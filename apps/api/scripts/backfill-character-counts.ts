#!/usr/bin/env node

// 글자 수 집계 단일화 이후 기존 문서의 text/characterCount 재계산. 각 워커가 `shardOf(documentId) % workers
// == workerIndex` 샤드를 맡아 스캔과 wasm 을 돌리고, 이 프로세스는 변경·실패 원장을 병합·기록하기만 한다.
// dry-run 이 기본이다 (--yes 없이는 문서 저장소에 아무것도 쓰지 않는다):
//
//   미리보기: SCRIPT=1 NODE_ENV=production doppler run --config prod_local -- \
//     node scripts/backfill-character-counts.ts 2>&1 | tee backfill-cc-dry.log
//   실행:     SCRIPT=1 NODE_ENV=production doppler run --config prod_local -- \
//     node scripts/backfill-character-counts.ts --yes [--workers <n>] [--batch <n>] [--enqueue-rate <n/s>] \
//     [--checkpoint <path>] 2>&1 | tee backfill-cc.log
//
// WASM_POOL_SIZE=1 DB_POOL_MAX=2 는 명령줄에서 뺐다 — 워커 spawn env 로 옮겼다.
//
// 문서별 재계산은 멱등이라 중단 후 재실행해도 안전하다 — 중단된 실행은 샤드 체크포인트의 커서에서 이어가며,
// 갱신되지 않은 실패 문서(예외는 아래 재색인 항목)는 그 커서보다 뒤(id 가 큰 쪽)에 남아 있는 한 다시 집힌다.
// 완주한 실행은 커서를 테이블 끝에 두므로 같은 체크포인트로 다시 돌리면 한 건도 스캔하지 않는다(scanned=0) —
// 전량 재스캔은 --checkpoint 를 새 경로로 주거나 기존 shard 파일을 다른 곳으로 옮긴 뒤에만 일어난다(파일을
// 삭제하지 말 것). 체크포인트와 원장은 dry/apply 별로 분리돼 있어, 완주한 미리보기 뒤 같은 --checkpoint 로
// --yes 를 주는 첫 실행은 언제나 전량 스캔이다.
// --workers 를 바꾸면 샤드가 재매핑되므로 하드 스톱한다.
//
// 변경 목록은 dry·apply 모두 backfill-cc-changes.<mode>.json 에 쌓인다(dry 는 변경 예정값) — 재색인 잡이
// 유실돼도 이 파일만으로 대상을 복원할 수 있는 복구 원장이다. 실패와 재색인 실패는 backfill-cc-report.<mode>.json
// 에 남고, 남아 있으면 종료 코드가 1 이다 — 워커가 done 회신에 스스로 종료하므로 종료 코드가 실효적이다.
//
// 실패·재색인 실패로 남은 문서는 재색인 상태를 별도로 확인한다 — 커밋 직후 응답이 유실돼 failed 로 잡힌
// 경우에도 값은 이미 갱신돼 있어 재실행이 동등 게이트(text/characterCount 일치)에 걸려 enqueue 를 다시 하지
// 않으며, 그 문서는 changes 원장에도 남지 않는다. 확인은 재실행 전에 한다 — 전량 재스캔(위 체크포인트 조건)으로
// 다시 돌리면 깨끗하게 재스캔된 failed 항목은 조용히 지워진다. reindex-failed 항목은 재실행이 지우지 않으니,
// 수동 재색인을 마쳤으면 backfill-cc-report.apply.json 에서 그 항목을 지운다. 그때까지 재실행이 exit 1 로
// 끝나는 것은 정상이다.
//
// 주의: 기동만으로(미리보기 포함) 프로덕션 큐의 크론 스케줄러 7건이 이 체크아웃의 정의로 재등록된다
// — 워커가 임포트하는 mq/index.ts:18-23 의 upsertJobScheduler 는 임포트 시점에 무조건 덮어쓴다. 반드시
// 배포본과 같은 리비전의 체크아웃에서 실행할 것.
//
// SCRIPT=1 은 필수다 — #/mq/index.ts 를 임포트하면 mq/bullmq.ts 가 평가 시점에 worker.run() 을 돌려
// 그 프로세스가 프로덕션 잡을 집어삼킨다(bullmq.ts:72-73). ESM 은 임포트를 본문보다 먼저 평가하므로 아래
// process.env.SCRIPT 세팅은 이 파일이 #/ 런타임 임포트를 하지 않는 동안에만 유효하다(워커에는 spawn env 로
// 전파된다 — 이 파일에 #/ 런타임 임포트를 추가하지 말 것). 프로세스 시작 전에 주입해야 한다.
//
// NODE_ENV=production 도 필수다 — 잡 레인 이름이 dev 면 os.hostname(), 아니면 stack 이다(bullmq.ts:10,
// env.ts:55). 워크스테이션에서 NODE_ENV 없이 돌리면 재색인 잡이 아무 워커도 소비하지 않는 hostname 레인에
// 쌓여 조용히 유실된다.
//
// --config prod_local 은 doppler 기본 config(dev_local) 를 덮기 위한 것이다 — 이 디렉토리의 프로덕션
// 스크립트 관례(verify-sweep-readiness.ts, sweep-zombie-documents.ts, migrate-comment-anchors.ts).
// 실제 config 명칭은 실행 승인 시 오너가 확인한다.

import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseArgs } from 'node:util';
import { Worker } from 'node:worker_threads';

process.env.SCRIPT = '1';

// 워커의 동명 타입과 짝이다 — 스크립트 파일은 export 를 둘 수 없어(unicorn/no-exports-in-scripts) 양쪽에 둔다.
type BackfillReportEntry = { documentId: string; reason: 'failed' | 'reindex-failed'; message: string };
type BackfillChange = { documentId: string; from: number; to: number };

const { values } = parseArgs({
  options: {
    yes: { type: 'boolean', default: false },
    checkpoint: { type: 'string', default: '.backfill-cc-checkpoint' },
    batch: { type: 'string', default: '500' },
    workers: { type: 'string' },
    'enqueue-rate': { type: 'string', default: '50' },
  },
});

const dryRun = !values.yes;
const mode = dryRun ? 'dry' : 'apply';
const checkpointBase = values.checkpoint;
const reportPath = `backfill-cc-report.${mode}.json`;
const changesPath = `backfill-cc-changes.${mode}.json`;
const batchSize = Number(values.batch);
const workerCount = Math.max(1, values.workers ? Number(values.workers) : os.availableParallelism() - 1);
const enqueueRate = Number(values['enqueue-rate']);

const FLUSH_EVERY = 2000;

const shardCheckpointPath = (shard: number): string => `${checkpointBase}.${mode}.shard-${shard}-of-${workerCount}`;
const legacyCheckpointPath = `${checkpointBase}.${mode}`;

const escapeRegExp = (value: string): string => value.replaceAll(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);

const readJsonArray = <T>(path: string): T[] => {
  if (!existsSync(path)) {
    return [];
  }
  try {
    const parsed: unknown = JSON.parse(readFileSync(path, 'utf8'));
    return Array.isArray(parsed) ? (parsed as T[]) : [];
  } catch (err) {
    console.warn(`${path}: 파싱 실패, 빈 목록으로 시작합니다 —`, err);
    return [];
  }
};

// SweepReportEntry 와 reason union 이 달라 #/utils/sweep-sharding.ts 의 병합 헬퍼를 그대로 쓸 수 없다.
const mergeById = <T extends { documentId: string }>(existing: readonly T[], incoming: readonly T[]): T[] => {
  const byDocument = new Map<string, T>();
  for (const item of existing) {
    byDocument.set(item.documentId, item);
  }
  for (const item of incoming) {
    byDocument.set(item.documentId, item);
  }
  return [...byDocument.values()];
};

// A different N remaps every document, so a shard layout only resumes under its own N:
// resume same-N, hard-stop on a different N, warn+restart for a stray single file.
const resolveShardCursors = (): string[] => {
  const dir = path.dirname(shardCheckpointPath(0)) || '.';
  const shardRe = new RegExp(String.raw`^${escapeRegExp(path.basename(checkpointBase))}\.${mode}\.shard-(\d+)-of-(\d+)$`);
  const existing = existsSync(dir) ? readdirSync(dir).filter((name) => shardRe.test(name)) : [];

  if (existing.length > 0) {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const foundCounts = new Set(existing.map((name) => Number(shardRe.exec(name)![2])));
    if (foundCounts.size > 1 || !foundCounts.has(workerCount)) {
      console.error(
        `기존 샤드 체크포인트의 워커 수(${[...foundCounts].join(', ')})가 현재 --workers ${workerCount}와 다릅니다.\n` +
          `같은 --workers 값으로 재실행하거나 기존 shard 파일을 다른 곳으로 옮긴 뒤 다시 시작하세요 (파일을 삭제하지 마세요).`,
      );
      process.exit(1);
    }
    const cursors = Array.from({ length: workerCount }, () => '');
    for (let shard = 0; shard < workerCount; shard++) {
      const path = shardCheckpointPath(shard);
      if (existsSync(path)) {
        const parsed = JSON.parse(readFileSync(path, 'utf8')) as { workers: number; cursor: string };
        cursors[shard] = parsed.cursor ?? '';
      }
    }
    console.log(`샤드 체크포인트 ${existing.length}개에서 이어갑니다 (workers=${workerCount}).`);
    return cursors;
  }

  if (existsSync(legacyCheckpointPath)) {
    console.warn(
      `단일 체크포인트(${legacyCheckpointPath})는 샤드 방식으로 이어갈 수 없습니다 — 무시하고 처음부터 시작합니다. ` +
        `${dryRun ? '(dry-run이라 안전합니다.)' : '(apply는 재실행이 필요합니다.)'} 기존 파일은 보존합니다.`,
    );
  }
  return Array.from({ length: workerCount }, () => '');
};

const shardCursors = resolveShardCursors();
let report = readJsonArray<BackfillReportEntry>(reportPath);
let changes = readJsonArray<BackfillChange>(changesPath);

// 이전 실행에서 failed 로 남은 문서들; 워커가 깨끗하게 재스캔한 id 를 회신하면 그 항목을 지운다. reindex-failed
// 는 값이 이미 갱신돼 깨끗한 재스캔이 재색인을 증명하지 못하므로 자동 해소 대상에서 뺀다.
const staleIds = report.filter((entry) => entry.reason === 'failed').map((entry) => entry.documentId);

// 리포트·변경 원장을 체크포인트보다 먼저 쓴다: 그 사이 크래시는 (체크포인트가 뒤처져 있으므로) 재스캔으로
// 끝나지만, 순서가 반대면 실패·변경 기록이 유실된다.
const flushReport = (): void => {
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
};

const flush = (): void => {
  flushReport();
  writeFileSync(changesPath, JSON.stringify(changes, null, 2));
  for (let shard = 0; shard < workerCount; shard++) {
    writeFileSync(shardCheckpointPath(shard), JSON.stringify({ workers: workerCount, cursor: shardCursors[shard] }));
  }
};

type WorkerMessage =
  | {
      type: 'result';
      cursor: string;
      entries: BackfillReportEntry[];
      changes: BackfillChange[];
      resolved: string[];
      stats: { scanned: number; changed: number; skippedDegraded: number; missing: number };
    }
  | { type: 'exhausted' }
  | { type: 'fatal'; message: string };

console.log(dryRun ? 'DRY RUN (실제 적용은 --yes)' : 'APPLY MODE');
console.log(`워커 ${workerCount}개, batch ${batchSize}${dryRun ? '' : `, enqueue-rate ${enqueueRate}/s`}`);

let scanned = 0;
let changed = 0;
let skippedDegraded = 0;
let missing = 0;
let sinceFlush = 0;
let aborted = false;

await new Promise<void>((resolve, reject) => {
  let active = 0;

  const spawn = (shard: number): void => {
    const worker = new Worker(new URL('backfill-character-counts-worker.ts', import.meta.url), {
      workerData: {
        workerIndex: shard,
        workers: workerCount,
        dryRun,
        batch: batchSize,
        startCursor: shardCursors[shard],
        enqueueRate,
        staleIds,
      },
      execArgv: process.execArgv,
      env: { ...process.env, WASM_POOL_SIZE: '1', DB_POOL_MAX: '2' },
    });
    active += 1;

    worker.on('message', (message: WorkerMessage) => {
      if (message.type === 'fatal') {
        reject(new Error(`worker ${shard}: ${message.message}`));
        return;
      }
      if (message.type === 'exhausted') {
        worker.postMessage({ done: true });
        return;
      }

      report = mergeById(report, message.entries);
      if (message.resolved.length > 0) {
        const cleared = new Set(message.resolved);
        report = report.filter((entry) => !cleared.has(entry.documentId));
      }
      if (message.entries.length > 0) {
        flushReport();
      }
      if (message.changes.length > 0) {
        changes = mergeById(changes, message.changes);
      }
      shardCursors[shard] = message.cursor;
      scanned += message.stats.scanned;
      changed += message.stats.changed;
      skippedDegraded += message.stats.skippedDegraded;
      missing += message.stats.missing;

      sinceFlush += message.stats.scanned;
      if (sinceFlush >= FLUSH_EVERY) {
        flush();
        sinceFlush = 0;
      }

      process.stdout.write(`\r스캔 ${scanned} 변경 ${changed} degraded ${skippedDegraded} 소실 ${missing} 미해결 ${report.length}   `);
    });

    worker.on('error', reject);
    worker.on('exit', () => {
      active -= 1;
      if (active === 0) {
        resolve();
      }
    });
  };

  for (let shard = 0; shard < workerCount; shard++) {
    spawn(shard);
  }
}).catch((err) => {
  aborted = true;
  console.error('\n워커 비정상 종료 — 부분 상태를 기록합니다:', err);
});

flush();
process.stdout.write('\n');
console.log(
  `scanned=${scanned} changed=${changed} skippedDegraded=${skippedDegraded} missing=${missing} unresolved=${report.length}${aborted ? ' ABORTED' : ''}`,
);

const reindexFailed = report.filter((entry) => entry.reason === 'reindex-failed').length;
if (reindexFailed > 0) {
  console.error(`수동 재색인 필요 ${reindexFailed}건 — 값은 갱신됐으나 재색인 잡이 등록되지 않았다 (${reportPath}).`);
}

process.exit(aborted || report.length > 0 ? 1 : 0);
