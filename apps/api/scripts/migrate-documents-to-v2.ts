#!/usr/bin/env node

import { writeFile } from 'node:fs/promises';
import os from 'node:os';
import { parseArgs } from 'node:util';
import { Worker } from 'node:worker_threads';
import { EntityState } from '@typie/lib/enums';
import { and, asc, count, eq, gt, isNull, ne } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, Documents, DocumentStates, Entities } from '#/db/index.ts';
import type { MigrateDocumentResult } from '#/utils/migrate-v2.ts';

process.env.SCRIPT = '1';

const BATCH_SIZE = 20;
const WINDOW_SIZE = 2000;

const { values } = parseArgs({
  options: {
    'dry-run': { type: 'boolean', default: false },
    concurrency: { type: 'string' },
    ids: { type: 'string' },
    'skip-drain-check': { type: 'boolean', default: false },
    profile: { type: 'boolean', default: false },
    limit: { type: 'string' },
  },
});

const dryRun = values['dry-run'] ?? false;
const profile = values.profile ?? false;
const limit = values.limit ? Number(values.limit) : null;
const concurrency = values.concurrency ? Number(values.concurrency) : os.cpus().length;

if (!values['skip-drain-check']) {
  const keys = await redis.keys('document:sync:updates:*');
  if (keys.length > 0) {
    if (dryRun) {
      console.warn(`경고: 미수집 legacy 업데이트 ${keys.length}건 잔존 — 해당 문서의 리허설 결과는 본실행 시점과 다를 수 있음.`);
    } else {
      console.error(`미수집 legacy 업데이트 ${keys.length}건 잔존. document:sync:collect 드레인 후 재시도하거나 --skip-drain-check 사용.`);
      process.exit(1);
    }
  }
}

const baseFilter = and(ne(Entities.state, EntityState.PURGED), isNull(DocumentStates.documentId));

const fetchWindow = async (afterId: string | null): Promise<string[]> =>
  await db
    .select({ id: Documents.id })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .leftJoin(DocumentStates, eq(DocumentStates.documentId, Documents.id))
    .where(afterId ? and(baseFilter, gt(Documents.id, afterId)) : baseFilter)
    .orderBy(asc(Documents.id))
    .limit(WINDOW_SIZE)
    .then((rows) => rows.map((row) => row.id));

const explicitIds = values.ids ? values.ids.split(',') : null;

const total = explicitIds
  ? explicitIds.length
  : await db
      .select({ value: count() })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .leftJoin(DocumentStates, eq(DocumentStates.documentId, Documents.id))
      .where(baseFilter)
      .then(([row]) => row.value);

console.log(`대상 문서 ${total}건, 워커 ${concurrency}개, dryRun=${dryRun}`);

const queue: string[] = explicitIds ? [...explicitIds] : [];
let lastId: string | null = null;
let exhausted = explicitIds !== null;
let refilling: Promise<void> | null = null;

const refill = async () => {
  const window = await fetchWindow(lastId);
  if (window.length === 0) {
    exhausted = true;
    return;
  }
  lastId = window.at(-1) ?? lastId;
  queue.push(...window);
};

let supplied = 0;

const nextBatch = async (): Promise<string[]> => {
  while (queue.length < BATCH_SIZE && !exhausted) {
    refilling ??= refill().finally(() => {
      refilling = null;
    });
    await refilling;
  }
  const budget = limit === null ? BATCH_SIZE : Math.min(BATCH_SIZE, Math.max(0, limit - supplied));
  const batch = queue.splice(0, budget);
  supplied += batch.length;
  return batch;
};

const startTime = Date.now();
let processed = 0;
const failures: MigrateDocumentResult[] = [];
const skips: MigrateDocumentResult[] = [];
const warned: MigrateDocumentResult[] = [];
const stageTotals: Record<string, number> = {};

const formatEta = (ms: number): string => {
  const seconds = Math.ceil(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
};

let aborted = false;

await new Promise<void>((resolve, reject) => {
  let active = 0;

  const spawn = () => {
    const worker = new Worker(new URL('migrate-documents-to-v2-worker.ts', import.meta.url), {
      workerData: { dryRun, profile, skipExistingCheck: explicitIds === null },
      execArgv: process.execArgv,
      env: { ...process.env, WASM_POOL_SIZE: '1', DB_POOL_MAX: '2' },
    });
    active += 1;

    worker.on('message', (results: MigrateDocumentResult[]) => {
      for (const result of results) {
        processed += 1;
        if (result.status === 'failed') failures.push(result);
        else if (result.status === 'skipped') skips.push(result);
        else {
          if (result.warnings.length > 0) warned.push(result);
          if (result.timings) {
            for (const [k, v] of Object.entries(result.timings)) {
              stageTotals[k] = (stageTotals[k] ?? 0) + v;
            }
          }
        }
      }
      if (results.length > 0) {
        const elapsed = Date.now() - startTime;
        const eta = processed > 0 ? formatEta(((total - processed) * elapsed) / processed) : '?';
        process.stdout.write(`\r${processed}/${total} 실패 ${failures.length} 스킵 ${skips.length} ETA ${eta}   `);
      }

      nextBatch()
        .then((batch) => {
          if (batch.length === 0) worker.postMessage({ done: true });
          else worker.postMessage({ ids: batch });
        })
        .catch(reject);
    });

    worker.on('error', reject);
    worker.on('exit', () => {
      active -= 1;
      if (active === 0) resolve();
    });
  };

  for (let i = 0; i < concurrency; i++) spawn();
}).catch((err) => {
  aborted = true;
  console.error('\n워커 비정상 종료 — 부분 리포트를 기록합니다:', err);
});

const report = {
  dryRun,
  aborted,
  total,
  processed,
  migrated: processed - failures.length - skips.length,
  failed: failures,
  skipped: skips,
  withWarnings: warned,
  elapsedMs: Date.now() - startTime,
  ...(profile && { stageTotalsMs: Object.fromEntries(Object.entries(stageTotals).map(([k, v]) => [k, Math.round(v)])) }),
};

const reportPath = `migration-report-${Date.now()}.json`;
await writeFile(reportPath, JSON.stringify(report, null, 2));
console.log(
  `\n완료. 리포트: ${reportPath} (실패 ${failures.length}, 스킵 ${skips.length}, 경고 ${warned.length}${aborted ? ', ABORTED' : ''})`,
);
if (profile) {
  const totalStage = Object.values(stageTotals).reduce((a, b) => a + b, 0);
  for (const [k, v] of Object.entries(stageTotals).toSorted(([, a], [, b]) => b - a)) {
    console.log(`  ${k.padEnd(24)} ${(v / 1000).toFixed(1)}s (${((v / totalStage) * 100).toFixed(0)}%)`);
  }
}
process.exit(aborted || failures.length > 0 ? 1 : 0);
