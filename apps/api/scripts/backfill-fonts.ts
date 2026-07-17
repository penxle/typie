#!/usr/bin/env node

import os from 'node:os';
import { parseArgs } from 'node:util';
import { Worker } from 'node:worker_threads';
import { GetObjectCommand, HeadObjectCommand } from '@aws-sdk/client-s3';
import { asc, eq } from 'drizzle-orm';
import { db, FontFamilies, Fonts } from '#/db/index.ts';
import * as aws from '#/external/aws.ts';
import { FONTS_BUCKET, objectExistsNonEmpty } from '#/utils/backfill-fonts.ts';
import { decompressZstd } from '#/utils/compression.ts';
import { sfntHasTable } from '#/utils/sfnt.ts';
import type { BackfillResult, BackfillTarget } from '#/utils/backfill-fonts.ts';

process.env.SCRIPT = '1';

const SCAN_CONCURRENCY = 16;

const { values } = parseArgs({
  options: {
    'dry-run': { type: 'boolean', default: false },
    verify: { type: 'boolean', default: false },
    'find-colr': { type: 'boolean', default: false },
    concurrency: { type: 'string' },
  },
});

const dryRun = values['dry-run'] ?? false;
const verify = values.verify ?? false;
const concurrency = values.concurrency ? Number(values.concurrency) : os.cpus().length;

const formatDuration = (ms: number): string => {
  const seconds = Math.ceil(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}m ${remainingSeconds}s`;
};

const logProgress = (label: string, processed: number, total: number, startTime: number) => {
  const elapsed = Date.now() - startTime;
  const rate = processed / elapsed;
  const remaining = total - processed;
  const eta = rate > 0 ? formatDuration(remaining / rate) : '?';
  const percent = total > 0 ? Math.round((processed / total) * 100) : 100;
  process.stdout.write(`\r${label}: ${processed}/${total} (${percent}%) 경과 ${formatDuration(elapsed)} ETA ${eta}  `);
};

const mapWithConcurrency = async <T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> => {
  const results: R[] = Array.from({ length: items.length });
  let cursor = 0;
  await Promise.all(
    Array.from({ length: Math.min(limit, items.length) }, async () => {
      while (cursor < items.length) {
        const index = cursor++;
        results[index] = await fn(items[index]);
      }
    }),
  );
  return results;
};

const objectExists = async (key: string): Promise<boolean> => {
  try {
    await aws.s3.send(new HeadObjectCommand({ Bucket: FONTS_BUCKET, Key: key }));
    return true;
  } catch (err) {
    if (err instanceof Error && err.name === 'NotFound') {
      return false;
    }
    throw err;
  }
};

const getObject = async (key: string): Promise<Uint8Array> => {
  const object = await aws.s3.send(new GetObjectCommand({ Bucket: FONTS_BUCKET, Key: key }));
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  return await object.Body!.transformToByteArray();
};

const rows = await db
  .select({
    id: Fonts.id,
    postScriptName: Fonts.postScriptName,
    path: Fonts.path,
    hash: Fonts.hash,
    chunks: Fonts.chunks,
    userId: FontFamilies.userId,
  })
  .from(Fonts)
  .innerJoin(FontFamilies, eq(Fonts.familyId, FontFamilies.id))
  .orderBy(asc(Fonts.id));

if (values['find-colr']) {
  const targets = rows.filter((row) => row.hash !== '');
  const findStart = Date.now();
  let checked = 0;
  const matches: { id: string; postScriptName: string; path: string; hash: string }[] = [];
  await mapWithConcurrency(targets, 8, async (row) => {
    const key = `fonts/${row.path}/${row.hash}/base`;
    try {
      // base 객체는 zstd 압축본 — 해제 후에만 sfnt 디렉토리 파싱이 유효하다.
      const base = await decompressZstd(await getObject(key));
      const hasColr = sfntHasTable(base, 'COLR');
      if (hasColr === null) {
        process.stdout.write(`\n판정 불가: ${row.id} (${row.path})\n`);
      } else if (hasColr) {
        matches.push(row);
      }
    } catch (err) {
      process.stdout.write(`\nbase 조회 실패: ${row.id} (${row.path}) — ${err instanceof Error ? err.message : String(err)}\n`);
    }
    checked += 1;
    logProgress('COLR 스캔', checked, targets.length, findStart);
  });
  console.log(`\n\nCOLR 폰트 ${matches.length}개:`);
  for (const row of matches.toSorted((a, b) => a.id.localeCompare(b.id))) {
    console.log(`${row.id}\t${row.postScriptName}\t${row.path}\t${row.hash}`);
  }
  process.exit(0);
}

if (verify) {
  console.log(`검증 시작: 전체 ${rows.length}개 행`);

  const verifyStart = Date.now();
  let checked = 0;

  const notMigrated: { id: string; path: string }[] = [];
  const cffSuspect: { id: string; path: string }[] = [];
  const incomplete: { id: string; path: string; reason: string }[] = [];

  await mapWithConcurrency(rows, SCAN_CONCURRENCY, async (row) => {
    if (row.hash === '') {
      notMigrated.push({ id: row.id, path: row.path });
      checked++;
      logProgress('검증', checked, rows.length, verifyStart);
      return;
    }

    const hashBase = `fonts/${row.path}/${row.hash}`;
    const manifestOk = await objectExistsNonEmpty(`${hashBase}/manifest.v1`);
    if (manifestOk) {
      if (!(await objectExistsNonEmpty(`${hashBase}/base`))) {
        incomplete.push({ id: row.id, path: row.path, reason: 'base missing or empty' });
      }
      const expectedChunks = (row.chunks as number[][]).length;
      const chunkChecks = await mapWithConcurrency(
        Array.from({ length: expectedChunks }, (_, i) => i),
        SCAN_CONCURRENCY,
        (id) => objectExistsNonEmpty(`${hashBase}/chunks/${id}`),
      );
      const missingCount = chunkChecks.filter((ok) => !ok).length;
      if (missingCount > 0) {
        incomplete.push({ id: row.id, path: row.path, reason: `${missingCount}/${expectedChunks} chunk objects missing or empty` });
      }
    } else {
      const hasChunkObjects = await objectExistsNonEmpty(`${hashBase}/chunks/0`);
      if (hasChunkObjects) {
        incomplete.push({ id: row.id, path: row.path, reason: 'manifest.v1 missing or empty' });
      } else {
        cffSuspect.push({ id: row.id, path: row.path });
      }
    }

    checked++;
    logProgress('검증', checked, rows.length, verifyStart);
  });

  process.stdout.write('\n');
  console.log(
    `검증 완료: 마이그레이션됨 ${rows.length - notMigrated.length}개 중 불충족 ${incomplete.length}개, cff-suspect ${cffSuspect.length}개, 미마이그레이션 ${notMigrated.length}개(게이트 제외)`,
  );

  if (notMigrated.length > 0) {
    console.log('미마이그레이션 목록(게이트 제외):');
    for (const r of notMigrated) {
      console.log(`- ${r.id} ${r.path}`);
    }
  }

  if (cffSuspect.length > 0) {
    console.log('cff-suspect 목록:');
    for (const r of cffSuspect) {
      console.log(`- ${r.id} ${r.path}`);
    }
  }

  if (incomplete.length > 0) {
    console.log('불충족 목록:');
    for (const r of incomplete) {
      console.log(`- ${r.id} ${r.path}: ${r.reason}`);
    }
  }

  process.exit(incomplete.length > 0 || cffSuspect.length > 0 ? 1 : 0);
}

console.log(`전체 폰트 ${rows.length}개 스캔 시작`);

const scanStart = Date.now();
let scanned = 0;

const scanTargets = await mapWithConcurrency(rows, SCAN_CONCURRENCY, async (row): Promise<BackfillTarget | null> => {
  const manifestV1Ok = row.hash !== '' && (await objectExistsNonEmpty(`fonts/${row.path}/${row.hash}/manifest.v1`));
  const baseOk = row.hash !== '' && (await objectExistsNonEmpty(`fonts/${row.path}/${row.hash}/base`));
  const chunkProbeNeeded = row.hash !== '' && !manifestV1Ok;
  const hasChunk0 = chunkProbeNeeded && (await objectExistsNonEmpty(`fonts/${row.path}/${row.hash}/chunks/0`));
  const needsV2 = row.hash === '' || !baseOk || (!manifestV1Ok && !hasChunk0);
  const needsLegacy = !(await objectExists(`fonts/${row.path}/manifest.json`));
  const needsManifest = baseOk && !manifestV1Ok && hasChunk0;
  scanned++;
  logProgress('스캔', scanned, rows.length, scanStart);
  return needsV2 || needsLegacy || needsManifest
    ? {
        row: { id: row.id, postScriptName: row.postScriptName, path: row.path, userId: row.userId },
        needsV2,
        needsLegacy,
        needsManifest,
      }
    : null;
});

const pending = scanTargets.filter((target) => target !== null);

process.stdout.write('\n');

const v2Count = pending.filter((t) => t.needsV2).length;
const legacyCount = pending.filter((t) => t.needsLegacy).length;
const manifestCount = pending.filter((t) => t.needsManifest).length;
console.log(
  `대상 ${pending.length}개 (완료 ${rows.length - pending.length}개 스킵) — v2 필요 ${v2Count}개, legacy 필요 ${legacyCount}개, manifest 필요 ${manifestCount}개`,
);

if (dryRun) {
  for (const { row, needsV2, needsLegacy, needsManifest } of pending) {
    console.log(
      `- ${row.id} ${row.path} (${[needsV2 && 'v2', needsLegacy && 'legacy', needsManifest && 'manifest'].filter(Boolean).join(', ')})`,
    );
  }
  process.exit(0);
}

const succeeded: BackfillResult[] = [];
const skipped: BackfillResult[] = [];
const failed: BackfillResult[] = [];
const startTime = Date.now();
let processed = 0;
let cursor = 0;
let aborted = false;

if (pending.length > 0) {
  const workerCount = Math.min(concurrency, pending.length);
  console.log(`워커 ${workerCount}개로 백필 시작`);

  await new Promise<void>((resolve, reject) => {
    let active = 0;

    const spawn = () => {
      const worker = new Worker(new URL('backfill-fonts-worker.ts', import.meta.url), {
        execArgv: process.execArgv,
        env: { ...process.env, WASM_POOL_SIZE: '1', DB_POOL_MAX: '2' },
      });
      active += 1;

      worker.on('message', (result: BackfillResult | null) => {
        if (result) {
          processed += 1;
          if (result.status === 'failed') {
            failed.push(result);
            process.stdout.write(`\n실패: ${result.id} (${result.path}) — ${result.reason}\n`);
          } else if (result.status === 'skipped') {
            skipped.push(result);
          } else {
            succeeded.push(result);
          }
          logProgress('백필', processed, pending.length, startTime);
        }

        if (cursor < pending.length) {
          worker.postMessage({ target: pending[cursor] });
          cursor += 1;
        } else {
          worker.postMessage({ done: true });
        }
      });

      worker.on('error', reject);
      worker.on('exit', () => {
        active -= 1;
        if (active === 0) resolve();
      });
    };

    for (let i = 0; i < workerCount; i++) spawn();
  }).catch((err) => {
    aborted = true;
    console.error('\n워커 비정상 종료:', err);
  });
}

process.stdout.write('\n');
console.log(
  `완료: 성공 ${succeeded.length}개, 스킵 ${skipped.length}개, 실패 ${failed.length}개, 대상외 ${rows.length - pending.length}개`,
);

if (skipped.length > 0) {
  console.log('스킵 목록:');
  for (const s of skipped) {
    console.log(`- ${s.id} ${s.path}: ${s.reason}`);
  }
}

if (failed.length > 0) {
  console.log('실패 목록:');
  for (const f of failed) {
    console.log(`- ${f.id} ${f.path}: ${f.reason}`);
  }
}

process.exit(aborted || failed.length > 0 ? 1 : 0);
