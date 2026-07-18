#!/usr/bin/env node

// Each worker owns the shard `shardOf(documentId) % workers == workerIndex` and runs its
// own scan + wasm; this process only merges and persists their reports. dry-run is the
// default (nothing written to the document store without --yes):
//
//   SCRIPT=1 doppler run --config prod_local -- node --experimental-strip-types \
//     scripts/sweep-zombie-documents.ts [--yes] [--workers <n>] [--batch <n>] [--enqueue-rate <n/s>] [--checkpoint <path>]

import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseArgs } from 'node:util';
import { Worker } from 'node:worker_threads';
import { mergeCommentHits, mergeReportEntries } from '#/utils/sweep-sharding.ts';
import type { SweepCommentHit, SweepReportEntry } from '#/utils/sweep-sharding.ts';

process.env.SCRIPT = '1';

const { values } = parseArgs({
  options: {
    yes: { type: 'boolean', default: false },
    checkpoint: { type: 'string', default: '.sweep-checkpoint' },
    batch: { type: 'string', default: '500' },
    workers: { type: 'string' },
    'enqueue-rate': { type: 'string', default: '50' },
  },
});

const dryRun = !values.yes;
const mode = dryRun ? 'dry' : 'apply';
const checkpointBase = values.checkpoint;
const reportPath = `sweep-report.${mode}.json`;
const commentHitsPath = `sweep-comment-hits.${mode}.json`;
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

// A different N remaps every document, so a shard layout only resumes under its own N:
// resume same-N, hard-stop on a different N, warn+restart for the serial-era single file.
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
      `기존 단일 체크포인트(${legacyCheckpointPath})는 샤드 방식으로 이어갈 수 없습니다 — 무시하고 처음부터 시작합니다. ` +
        `${dryRun ? '(dry-run이라 안전합니다.)' : '(apply는 재실행이 필요합니다.)'} 기존 파일은 보존합니다.`,
    );
  }
  return Array.from({ length: workerCount }, () => '');
};

const shardCursors = resolveShardCursors();
let report = readJsonArray<SweepReportEntry>(reportPath);
let commentHits = readJsonArray<SweepCommentHit>(commentHitsPath);

// Snapshot of documents that carry a stale failed/deferred entry from a prior run; workers
// report which of them re-scan clean so their entries are dropped instead of lingering.
const staleIds = report.map((entry) => entry.documentId);

// Report/comment-hits before the checkpoints: a crash between the two must re-scan
// (checkpoint still behind) rather than lose a failed/deferred entry.
const flush = (): void => {
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  if (dryRun) {
    writeFileSync(commentHitsPath, JSON.stringify(commentHits, null, 2));
  }
  for (let shard = 0; shard < workerCount; shard++) {
    writeFileSync(shardCheckpointPath(shard), JSON.stringify({ workers: workerCount, cursor: shardCursors[shard] }));
  }
};

type WorkerMessage =
  | {
      type: 'result';
      cursor: string;
      entries: SweepReportEntry[];
      hits: SweepCommentHit[];
      resolved: string[];
      stats: { scanned: number; dirty: number; zombies: number; applied: number };
    }
  | { type: 'exhausted' }
  | { type: 'fatal'; message: string };

console.log(dryRun ? 'DRY RUN (실제 적용은 --yes)' : 'APPLY MODE');
console.log(`워커 ${workerCount}개, batch ${batchSize}${dryRun ? '' : `, enqueue-rate ${enqueueRate}/s`}`);

let scanned = 0;
let dirty = 0;
let totalZombies = 0;
let applied = 0;
let sinceFlush = 0;
let aborted = false;

await new Promise<void>((resolve, reject) => {
  let active = 0;

  const spawn = (shard: number): void => {
    const worker = new Worker(new URL('sweep-zombie-documents-worker.ts', import.meta.url), {
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

      report = mergeReportEntries(report, message.entries);
      if (message.resolved.length > 0) {
        const cleared = new Set(message.resolved);
        report = report.filter((entry) => !cleared.has(entry.documentId));
      }
      if (dryRun && message.hits.length > 0) {
        commentHits = mergeCommentHits(commentHits, message.hits);
      }
      shardCursors[shard] = message.cursor;
      scanned += message.stats.scanned;
      dirty += message.stats.dirty;
      totalZombies += message.stats.zombies;
      applied += message.stats.applied;

      sinceFlush += message.stats.scanned;
      if (sinceFlush >= FLUSH_EVERY) {
        flush();
        sinceFlush = 0;
      }

      process.stdout.write(
        `\r스캔 ${scanned} dirty ${dirty} zombies ${totalZombies}${dryRun ? '' : ` applied ${applied}`} 미해결 ${report.length}   `,
      );
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
  `scanned=${scanned} dirty=${dirty} totalZombies=${totalZombies}${dryRun ? '' : ` applied=${applied}`} unresolved=${report.length}${aborted ? ' ABORTED' : ''}`,
);
process.exit(aborted || report.length > 0 ? 1 : 0);
