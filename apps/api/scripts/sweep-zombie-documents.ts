#!/usr/bin/env node

// sweepDocument (#/utils/zombie-sweep.ts)는 #/mq/index.ts를 정적 import하고, 그 import가
// mq/bullmq.ts 평가 시점에 worker.run()을 실행한다(SCRIPT 환경변수가 없으면). ESM은 import를
// 스크립트 본문보다 먼저 평가하므로 여기서 process.env.SCRIPT를 설정해도 이미 늦다 — 프로세스
// 시작 전에 주입해야 한다:
//
//   SCRIPT=1 doppler run --config prod_local -- node --experimental-strip-types scripts/sweep-zombie-documents.ts [--yes] [--checkpoint <path>] [--batch <n>]
//
// dry-run이 기본값이며 --yes 없이는 DB에 아무 것도 쓰지 않는다.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';
import { asc, eq, gt } from 'drizzle-orm';
import { db, DocumentCommentThreads, DocumentStates } from '#/db/index.ts';
import { extractSelectionDots } from '#/utils/comment-selection.ts';
import { sweepDocument } from '#/utils/zombie-sweep.ts';

const { values } = parseArgs({
  options: {
    yes: { type: 'boolean', default: false },
    checkpoint: { type: 'string', default: '.sweep-checkpoint' },
    batch: { type: 'string', default: '100' },
  },
});

const dryRun = !values.yes;
const checkpointPath = `${values.checkpoint}.${dryRun ? 'dry' : 'apply'}`;
const reportPath = `sweep-report.${dryRun ? 'dry' : 'apply'}.json`;
const commentHitsPath = `sweep-comment-hits.${dryRun ? 'dry' : 'apply'}.json`;
const batchSize = Number(values.batch);

let cursor = existsSync(checkpointPath) ? readFileSync(checkpointPath, 'utf8').trim() : '';

type ReportEntry = { documentId: string; reason: 'failed' | 'deferred'; message?: string };
type CommentHit = { documentId: string; threadId: string; hitDots: string[] | null };

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

const report: ReportEntry[] = readJsonArray<ReportEntry>(reportPath);
const commentHits: CommentHit[] = readJsonArray<CommentHit>(commentHitsPath);

console.log(dryRun ? 'DRY RUN (실제 적용은 --yes)' : 'APPLY MODE');

const findZombieAnchoredComments = async (documentId: string, zombieDots: string[]): Promise<CommentHit[]> => {
  if (zombieDots.length === 0) {
    return [];
  }
  const zombieSet = new Set(zombieDots);

  const threads = await db
    .select({ id: DocumentCommentThreads.id, selection: DocumentCommentThreads.selection })
    .from(DocumentCommentThreads)
    .where(eq(DocumentCommentThreads.documentId, documentId));

  const hits: CommentHit[] = [];
  for (const thread of threads) {
    const extraction = extractSelectionDots(thread.selection);
    if (extraction.kind === 'unrecognized') {
      console.warn(`${documentId}: comment ${thread.id} selection 형식 미상 — 좀비 교차 검사 불가`);
      hits.push({ documentId, threadId: thread.id, hitDots: null });
      continue;
    }

    const hitDots = extraction.dots.filter((dot) => zombieSet.has(dot));
    if (hitDots.length > 0) {
      console.log(`${documentId}: comment ${thread.id} anchors on zombie dots [${hitDots.join(', ')}]`);
      hits.push({ documentId, threadId: thread.id, hitDots });
    }
  }
  return hits;
};

let scanned = 0;
let dirty = 0;
let totalZombies = 0;

for (;;) {
  const rows = await db
    .select({ documentId: DocumentStates.documentId })
    .from(DocumentStates)
    .where(cursor ? gt(DocumentStates.documentId, cursor) : undefined)
    .orderBy(asc(DocumentStates.documentId))
    .limit(batchSize);
  if (rows.length === 0) {
    break;
  }

  for (const { documentId } of rows) {
    scanned += 1;
    try {
      const result = await sweepDocument(documentId, { dryRun });
      if (result.deferred) {
        report.push({ documentId, reason: 'deferred' });
      } else if (result.deleteRunCount > 0) {
        dirty += 1;
        totalZombies += result.zombieDots.length;
        console.log(`${documentId}: zombies=${result.zombieDots.length} runs=${result.deleteRunCount} applied=${result.applied}`);
        if (dryRun) {
          commentHits.push(...(await findZombieAnchoredComments(documentId, result.zombieDots)));
        }
      }
    } catch (err) {
      console.error(`${documentId}: FAILED`, err);
      report.push({ documentId, reason: 'failed', message: err instanceof Error ? err.message : String(err) });
    }
    cursor = documentId;
    await new Promise((r) => setTimeout(r, 50));
  }

  // report/commentHits must land before the checkpoint — a crash between the two
  // must re-scan the batch rather than silently drop its failed/deferred entries.
  writeFileSync(reportPath, JSON.stringify(report, null, 2));
  if (dryRun) {
    writeFileSync(commentHitsPath, JSON.stringify(commentHits, null, 2));
  }
  writeFileSync(checkpointPath, cursor);
}

console.log(`scanned=${scanned} dirty=${dirty} totalZombies=${totalZombies} unresolved=${report.length}`);
process.exit(report.length > 0 ? 1 : 0);
