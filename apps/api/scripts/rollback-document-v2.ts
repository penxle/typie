#!/usr/bin/env node

import { parseArgs } from 'node:util';
import { eq, inArray } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentBundles,
  DocumentChangesetsDeadLetter,
  DocumentCommentThreads,
  DocumentHeadContributors,
  DocumentHeads,
  DocumentStates,
  first,
} from '#/db/index.ts';
import { Lock } from '#/lock.ts';
import { collectedKey, liveKey, streamKey } from '#/utils/changeset.ts';

process.env.SCRIPT = '1';

const { values, positionals } = parseArgs({
  options: { yes: { type: 'boolean', default: false } },
  allowPositionals: true,
});

const [documentId] = positionals;

if (!documentId || positionals.length !== 1 || !values.yes) {
  console.error('Usage: node scripts/rollback-document-v2.ts <documentId> --yes');
  console.error('경고: v2 전환 이후 발생한 편집과 모든 comment 스레드(이관분 포함)가 삭제됩니다.');
  console.error('실행 전 해당 문서를 연 v2 세션(브라우저 탭)을 모두 닫으세요. 컷오버 이후 사용 금지.');
  process.exit(1);
}

const state = await db
  .select({ documentId: DocumentStates.documentId })
  .from(DocumentStates)
  .where(eq(DocumentStates.documentId, documentId))
  .then(first);

if (!state) {
  console.error(`Document ${documentId} is not a v2 document (no DocumentStates row). Nothing to roll back.`);
  process.exit(1);
}

const documentLock = new Lock(`document:${documentId}`);
const changesetLock = new Lock(`document:changesets:${documentId}`);

if (!(await documentLock.acquire())) {
  console.error('락 획득 실패 — collect/GC 잡이 진행 중일 수 있습니다. 잠시 후 재시도하세요.');
  process.exit(1);
}

if (!(await changesetLock.acquire())) {
  await documentLock.release();
  console.error('락 획득 실패 — collect/GC 잡이 진행 중일 수 있습니다. 잠시 후 재시도하세요.');
  process.exit(1);
}

try {
  await db.transaction(async (tx) => {
    const heads = await tx.select({ id: DocumentHeads.id }).from(DocumentHeads).where(eq(DocumentHeads.documentId, documentId));
    if (heads.length > 0) {
      await tx.delete(DocumentHeadContributors).where(
        inArray(
          DocumentHeadContributors.headId,
          heads.map((h) => h.id),
        ),
      );
      await tx.delete(DocumentHeads).where(eq(DocumentHeads.documentId, documentId));
    }
    await tx.delete(DocumentCommentThreads).where(eq(DocumentCommentThreads.documentId, documentId));
    await tx.delete(DocumentChangesetsDeadLetter).where(eq(DocumentChangesetsDeadLetter.documentId, documentId));
    await tx.delete(DocumentStates).where(eq(DocumentStates.documentId, documentId));
    await tx.delete(DocumentBundles).where(eq(DocumentBundles.documentId, documentId));
  });

  await redis.del(streamKey(documentId), collectedKey(documentId), liveKey(documentId));
} finally {
  await changesetLock.release();
  await documentLock.release();
}

console.log(`Document ${documentId} rolled back to legacy.`);
process.exit(0);
