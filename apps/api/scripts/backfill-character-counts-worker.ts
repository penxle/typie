#!/usr/bin/env node

import { setTimeout as sleep } from 'node:timers/promises';
import { parentPort, workerData } from 'node:worker_threads';
import { asc, eq, gt, sql } from 'drizzle-orm';
import { db, DocumentStates, first } from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { shardOf } from '#/utils/sweep-sharding.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import type { PlainDoc } from '@typie/editor-ffi/server';

process.env.SCRIPT = '1';

// 메인의 동명 타입과 짝이다 — 스크립트 파일은 export 를 둘 수 없어(unicorn/no-exports-in-scripts) 양쪽에 둔다.
type BackfillReportEntry = { documentId: string; reason: 'failed' | 'reindex-failed'; message: string };
type BackfillChange = { documentId: string; from: number; to: number };

type WorkerData = {
  workerIndex: number;
  workers: number;
  dryRun: boolean;
  batch: number;
  startCursor: string;
  enqueueRate: number;
  staleIds: string[];
};

const { workerIndex, workers, dryRun, batch, startCursor, enqueueRate, staleIds } = workerData as WorkerData;

// 이전 실행의 실패 항목을 달고 있는 문서들 — 깨끗하게 재스캔되면 메인이 그 항목을 지운다.
const staleSet = new Set(staleIds);

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const port = parentPort!;

// 워커당 rate / workers 로 나눠 합계 enqueue 속도를 상한 아래로 유지한다.
const throttleActive = !dryRun && enqueueRate > 0;
const perWorkerRate = enqueueRate / workers;
let tokens = perWorkerRate;
let lastRefill = Date.now();

const refillTokens = (): void => {
  const now = Date.now();
  tokens = Math.min(perWorkerRate, tokens + ((now - lastRefill) / 1000) * perWorkerRate);
  lastRefill = now;
};

const throttleEnqueue = async (): Promise<void> => {
  refillTokens();
  tokens -= 1;
  while (tokens < 0) {
    await sleep(Math.min(1000, Math.ceil((-tokens / perWorkerRate) * 1000)));
    refillTokens();
  }
};

type DocOutcome = {
  entry: BackfillReportEntry | null;
  change: BackfillChange | null;
  degraded: boolean;
  missing: boolean;
};

const processDocument = async (documentId: string): Promise<DocOutcome> => {
  try {
    const outcome = await db.transaction(async (tx) => {
      // collect와 같은 락으로 fold 중간 상태 위에 덮어쓰는 경쟁을 차단한다.
      await tx.execute(sql`SELECT pg_advisory_xact_lock(hashtextextended(${documentId}, 0))`);

      const state = await tx
        .select({
          json: DocumentStates.json,
          text: DocumentStates.text,
          characterCount: DocumentStates.characterCount,
          projectionDegraded: DocumentStates.projectionDegraded,
        })
        .from(DocumentStates)
        .where(eq(DocumentStates.documentId, documentId))
        .then(first);

      if (!state) {
        return { change: null, degraded: false, missing: true };
      }
      if (state.projectionDegraded) {
        return { change: null, degraded: true, missing: false };
      }

      const { text, characterCount } = await wasmFfi.use((host) => {
        const text = host.extract_text(state.json as PlainDoc);
        return { text, characterCount: host.count_characters(text) };
      });

      if (text === state.text && characterCount === state.characterCount) {
        return { change: null, degraded: false, missing: false };
      }

      const change: BackfillChange = { documentId, from: state.characterCount, to: characterCount };
      if (dryRun) {
        return { change, degraded: false, missing: false };
      }

      await tx.update(DocumentStates).set({ text, characterCount }).where(eq(DocumentStates.documentId, documentId));
      return { change, degraded: false, missing: false };
    });

    if (!dryRun && outcome.change) {
      if (throttleActive) {
        await throttleEnqueue();
      }
      try {
        await enqueueJob('search:index:document', documentId);
      } catch (err) {
        // 값은 이미 커밋됐다 — 재실행은 동등 게이트에 걸려 enqueue 를 다시 하지 않으므로 별도 항목으로 남긴다.
        return {
          entry: { documentId, reason: 'reindex-failed', message: err instanceof Error ? err.message : String(err) },
          change: outcome.change,
          degraded: outcome.degraded,
          missing: outcome.missing,
        };
      }
    }

    return { entry: null, change: outcome.change, degraded: outcome.degraded, missing: outcome.missing };
  } catch (err) {
    // 문서 1건의 실패로 전체를 멈추지 않는다 — 갱신되지 않았으니 중단 재개나 전량 재스캔이 이 문서를 다시 집는다.
    return {
      entry: { documentId, reason: 'failed', message: err instanceof Error ? err.message : String(err) },
      change: null,
      degraded: false,
      missing: false,
    };
  }
};

const run = async (): Promise<void> => {
  let cursor = startCursor;
  for (;;) {
    const rows = await db
      .select({ documentId: DocumentStates.documentId })
      .from(DocumentStates)
      .where(cursor ? gt(DocumentStates.documentId, cursor) : undefined)
      .orderBy(asc(DocumentStates.documentId))
      .limit(batch);
    if (rows.length === 0) {
      break;
    }

    const entries: BackfillReportEntry[] = [];
    const changes: BackfillChange[] = [];
    const resolved: string[] = [];
    let scanned = 0;
    let changed = 0;
    let skippedDegraded = 0;
    let missing = 0;

    for (const { documentId } of rows) {
      cursor = documentId;
      if (shardOf(documentId, workers) !== workerIndex) {
        continue;
      }
      scanned += 1;
      const outcome = await processDocument(documentId);
      if (outcome.entry) {
        entries.push(outcome.entry);
      } else if (staleSet.has(documentId)) {
        resolved.push(documentId);
      }
      if (outcome.change) {
        changes.push(outcome.change);
        changed += 1;
      }
      if (outcome.degraded) {
        skippedDegraded += 1;
      }
      if (outcome.missing) {
        missing += 1;
      }
    }

    // cursor = 마지막으로 방문한 id (샤드 무관); 재시작은 `gt` 로 이어가 JS/DB collation 불일치 위험을 피한다.
    port.postMessage({ type: 'result', cursor, entries, changes, resolved, stats: { scanned, changed, skippedDegraded, missing } });
  }

  port.postMessage({ type: 'exhausted' });
};

port.on('message', (message: { done?: boolean }) => {
  if (message.done) {
    process.exit(0);
  }
});

try {
  await run();
} catch (err) {
  port.postMessage({ type: 'fatal', message: err instanceof Error ? err.message : String(err) });
}
