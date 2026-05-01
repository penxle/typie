import { DocumentConflictKind } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, eq, gt, inArray, isNotNull, lte, sql } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentCommits,
  DocumentConflictBranches,
  DocumentConflicts,
  DocumentHeadContents,
  DocumentObjects,
  Documents,
  first,
  firstOrThrow,
} from '#/db/index.ts';
import { Lock } from '#/lock.ts';
import { pubsub } from '#/pubsub.ts';
import { calculateBlobSizeFromAssetIds, countCharacters } from '#/utils/entity.ts';
import { loadDocFromRootObjectId, walkReachableHashes, walkReachableObjects } from '#/utils/sync.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { enqueueJob } from '../index.ts';
import { defineCron, defineJob } from '../types.ts';
import type { ConflictRecord, ObjectContent } from '@typie/editor-ffi/server';
import type { Transaction } from '#/db/index.ts';

type AdvanceHeadPayload = { documentId: string; leafCandidateId?: string };

export const DocumentAdvanceHeadJob = defineJob('document:advance-head', async (payload: AdvanceHeadPayload) => {
  const { documentId, leafCandidateId } = payload;
  const lock = new Lock(`document:${documentId}`);
  if (!(await lock.tryAcquire())) return;
  try {
    await processDocument(documentId, leafCandidateId, lock.signal);
  } finally {
    await lock.release();
  }
});

export const DocumentAdvanceHeadScanCron = defineCron('document:advance-head:scan', '* * * * *', async () => {
  const docs = await db.select({ id: Documents.id }).from(Documents).where(isNotNull(Documents.dirtyAt));
  await Promise.all(docs.map((d) => enqueueJob('document:advance-head', { documentId: d.id })));
});

async function processDocument(documentId: string, leafCandidateId: string | undefined, signal: AbortSignal): Promise<void> {
  const initial = await db.select({ dirtyAt: Documents.dirtyAt }).from(Documents).where(eq(Documents.id, documentId)).then(firstOrThrow);
  const dirtyAtSnapshot = initial.dirtyAt;
  if (dirtyAtSnapshot === null) return;

  signal.throwIfAborted();

  const { newHead, oldHead, deltaCommits, deltaObjects, cleared } = await db.transaction(async (tx) => {
    const docInfo = await tx
      .select({
        headCommitId: Documents.headCommitId,
        headSequence: DocumentCommits.sequence,
        headRootObjectId: DocumentCommits.rootObjectId,
      })
      .from(Documents)
      .leftJoin(DocumentCommits, eq(DocumentCommits.id, Documents.headCommitId))
      .where(eq(Documents.id, documentId))
      .then(firstOrThrow);

    const oldHead =
      docInfo.headCommitId && docInfo.headSequence !== null && docInfo.headRootObjectId !== null
        ? { commitId: docInfo.headCommitId, sequence: docInfo.headSequence, rootObjectId: docInfo.headRootObjectId }
        : null;
    const oldHeadCommitId = oldHead?.commitId ?? null;

    let head: string | null = oldHeadCommitId;
    let fastForward: Awaited<ReturnType<typeof tryFastForward>> = null;

    if (leafCandidateId) {
      fastForward = await tryFastForward(tx, documentId, oldHead, leafCandidateId);
    }

    let candidates: (typeof DocumentCommits.$inferSelect)[] | null = null;
    if (!fastForward) {
      candidates = await findCandidateLeaves(tx, documentId, oldHeadCommitId);
      if (!leafCandidateId && candidates.length === 1 && candidates[0].secondParentId === null) {
        fastForward = await tryFastForward(tx, documentId, oldHead, candidates[0].id);
      }
    }

    if (fastForward) {
      head = fastForward.leaf.id;
    } else if (candidates && candidates.length > 0) {
      const ordered = candidates.toSorted((a, b) => a.sequence - b.sequence);

      for (const leaf of ordered) {
        signal.throwIfAborted();
        if (head === null) {
          head = leaf.id;
          continue;
        }
        const lcaId = await computeLCA(tx, head, leaf.id);
        if (lcaId === head) {
          head = leaf.id;
        } else if (lcaId === leaf.id) {
          continue;
        } else {
          head = await performMerge(tx, documentId, lcaId, head, leaf.id);
        }
      }
    }

    let deltaCommits: (typeof DocumentCommits.$inferSelect)[] = [];
    let deltaObjects: (typeof DocumentObjects.$inferSelect)[] = [];

    if (head !== null && head !== oldHeadCommitId) {
      const newHashes = await updateHead(tx, documentId, head, fastForward?.leaf.rootObjectId);
      if (fastForward) {
        deltaCommits = fastForward.chainCommits;
        const oldHashes = oldHead ? await walkReachableHashes(tx, oldHead.rootObjectId) : new Set<string>();
        const deltaHashes = [...newHashes].filter((h) => !oldHashes.has(h));
        deltaObjects =
          deltaHashes.length > 0 ? await tx.select().from(DocumentObjects).where(inArray(DocumentObjects.hash, deltaHashes)) : [];
      } else {
        ({ deltaCommits, deltaObjects } = await collectDelta(tx, documentId, oldHeadCommitId, head));
      }
    }

    const clearResult = await tx
      .update(Documents)
      .set({ dirtyAt: null })
      .where(and(eq(Documents.id, documentId), eq(Documents.dirtyAt, dirtyAtSnapshot)))
      .returning({ id: Documents.id });

    return {
      newHead: head,
      oldHead: oldHeadCommitId,
      deltaCommits,
      deltaObjects,
      cleared: clearResult.length > 0,
    };
  });

  if (newHead !== oldHead && newHead !== null) {
    pubsub.publish('document:commits', documentId, {
      commitIds: deltaCommits.map((c) => c.id),
      objectIds: deltaObjects.map((o) => o.id),
    });
  }

  if (!cleared) {
    await enqueueJob('document:advance-head', { documentId });
  }
}

async function tryFastForward(
  tx: Transaction,
  documentId: string,
  oldHead: { commitId: string; sequence: number; rootObjectId: string } | null,
  leafCandidateId: string,
): Promise<{
  leaf: typeof DocumentCommits.$inferSelect;
  chainCommits: (typeof DocumentCommits.$inferSelect)[];
} | null> {
  const oldHeadSequence = oldHead?.sequence ?? -1;
  const oldHeadCommitId = oldHead?.commitId ?? null;

  const leafSeqRow = await tx
    .select({ sequence: DocumentCommits.sequence, secondParentId: DocumentCommits.secondParentId })
    .from(DocumentCommits)
    .where(and(eq(DocumentCommits.documentId, documentId), eq(DocumentCommits.id, leafCandidateId)))
    .then(first);
  if (!leafSeqRow) return null;
  if (leafSeqRow.secondParentId !== null) return null;
  if (leafSeqRow.sequence <= oldHeadSequence) return null;

  const rangeCommits = await tx
    .select()
    .from(DocumentCommits)
    .where(
      and(
        eq(DocumentCommits.documentId, documentId),
        gt(DocumentCommits.sequence, oldHeadSequence),
        lte(DocumentCommits.sequence, leafSeqRow.sequence),
      ),
    )
    .orderBy(asc(DocumentCommits.sequence));

  const leaf = rangeCommits.find((c) => c.id === leafCandidateId);
  if (!leaf) return null;

  const byId = new Map(rangeCommits.map((c) => [c.id, c]));
  const chainCommits: (typeof DocumentCommits.$inferSelect)[] = [];
  let cursor: typeof DocumentCommits.$inferSelect | undefined = leaf;
  while (cursor) {
    if (cursor.secondParentId !== null) return null;
    chainCommits.push(cursor);
    if (cursor.parentId === oldHeadCommitId) {
      chainCommits.reverse();
      return { leaf, chainCommits };
    }
    if (cursor.parentId === null) return null;
    cursor = byId.get(cursor.parentId);
  }
  return null;
}

async function findCandidateLeaves(
  tx: Transaction,
  documentId: string,
  headCommitId: string | null,
): Promise<(typeof DocumentCommits.$inferSelect)[]> {
  const headAncestors = headCommitId ? await collectAncestorIds(tx, headCommitId) : new Set<string>();

  const leaves = await tx
    .select()
    .from(DocumentCommits)
    .where(
      and(
        eq(DocumentCommits.documentId, documentId),
        sql`
          NOT EXISTS (
                    SELECT 1 FROM ${DocumentCommits} c2
                    WHERE c2.document_id = ${documentId}
                    AND (c2.parent_id = ${DocumentCommits.id} OR c2.second_parent_id = ${DocumentCommits.id})
                  )
        `,
      ),
    );

  return leaves.filter((l) => !headAncestors.has(l.id));
}

async function computeLCA(tx: Transaction, a: string, b: string): Promise<string> {
  if (a === b) return a;

  const ancestorsA = await collectAncestorIds(tx, a);
  if (ancestorsA.has(b)) return b;

  const rows = await tx.execute<{ id: string }>(sql`
    WITH RECURSIVE ancestors AS (
      SELECT id, parent_id, second_parent_id, 0 AS depth FROM ${DocumentCommits} WHERE id = ${b}
      UNION
      SELECT c.id, c.parent_id, c.second_parent_id, ab.depth + 1
      FROM ${DocumentCommits} c
      JOIN ancestors ab ON c.id = ab.parent_id OR c.id = ab.second_parent_id
    )
    SELECT id FROM ancestors GROUP BY id ORDER BY MIN(depth) ASC
  `);

  for (const row of rows) {
    if (ancestorsA.has(row.id)) return row.id;
  }

  throw new Error('no common ancestor between commits');
}

async function collectAncestorIds(tx: Transaction, commitId: string): Promise<Set<string>> {
  const rows = await tx.execute<{ id: string }>(sql`
    WITH RECURSIVE ancestors AS (
      SELECT id, parent_id, second_parent_id FROM ${DocumentCommits} WHERE id = ${commitId}
      UNION
      SELECT c.id, c.parent_id, c.second_parent_id
      FROM ${DocumentCommits} c
      JOIN ancestors a ON c.id = a.parent_id OR c.id = a.second_parent_id
    )
    SELECT id FROM ancestors
  `);
  return new Set(rows.map((r) => r.id));
}

async function performMerge(tx: Transaction, documentId: string, baseId: string, oursId: string, theirsId: string): Promise<string> {
  const baseCommit = await tx.select().from(DocumentCommits).where(eq(DocumentCommits.id, baseId)).then(firstOrThrow);
  const oursCommit = await tx.select().from(DocumentCommits).where(eq(DocumentCommits.id, oursId)).then(firstOrThrow);
  const theirsCommit = await tx.select().from(DocumentCommits).where(eq(DocumentCommits.id, theirsId)).then(firstOrThrow);

  const { doc: baseDoc } = await loadDocFromRootObjectId(tx, baseCommit.rootObjectId);
  const { doc: oursDoc } = await loadDocFromRootObjectId(tx, oursCommit.rootObjectId);
  const { doc: theirsDoc } = await loadDocFromRootObjectId(tx, theirsCommit.rootObjectId);

  const mergeResult = await wasm.merge_docs(baseDoc, oursDoc, theirsDoc);
  const { rootHash: mergedRootHash, objects: mergedAllObjects } = await wasm.derive_all_objects(mergeResult.merged);

  if (mergedAllObjects.length > 0) {
    await tx
      .insert(DocumentObjects)
      .values(mergedAllObjects.map((o) => ({ hash: o.hash, content: o.content })))
      .onConflictDoNothing({ target: DocumentObjects.hash });
  }

  const mergedRootObj = await tx
    .select({ id: DocumentObjects.id })
    .from(DocumentObjects)
    .where(eq(DocumentObjects.hash, mergedRootHash))
    .then(firstOrThrow);

  // Server-issued merge commit: deviceId/userId null, distinguished by meta.merge=true.
  const mergeHash = await wasm.hash_commit_content({
    parent_hash: oursCommit.hash,
    second_parent_hash: theirsCommit.hash,
    object_hash: mergedRootHash,
  });

  const inserted = await tx
    .insert(DocumentCommits)
    .values({
      hash: mergeHash,
      documentId,
      parentId: oursId,
      secondParentId: theirsId,
      rootObjectId: mergedRootObj.id,
      steps: null,
      meta: { merge: true },
      deviceId: null,
      userId: null,
      committedAt: dayjs(),
    })
    .onConflictDoNothing({ target: [DocumentCommits.documentId, DocumentCommits.hash] })
    .returning()
    .then(first);

  let mergeCommit: typeof DocumentCommits.$inferSelect;
  if (inserted) {
    mergeCommit = inserted;
    await persistConflicts(tx, documentId, mergeCommit.id, oursId, theirsId, mergeResult.conflicts);
  } else {
    // 같은 머지가 이미 처리됐다면 conflicts도 이미 기록됐으므로 중복 호출 회피
    mergeCommit = await tx
      .select()
      .from(DocumentCommits)
      .where(and(eq(DocumentCommits.documentId, documentId), eq(DocumentCommits.hash, mergeHash)))
      .then(firstOrThrow);
  }

  return mergeCommit.id;
}

async function persistConflicts(
  tx: Transaction,
  documentId: string,
  mergeCommitId: string,
  oursCommitDbId: string,
  theirsCommitDbId: string,
  conflicts: ConflictRecord[],
): Promise<void> {
  for (const conflict of conflicts) {
    const conflictRow = await tx
      .insert(DocumentConflicts)
      .values({
        documentId,
        mergeCommitId,
        kind: conflict.kind.toUpperCase() as DocumentConflictKind,
        target: conflict.target,
        baseValue: conflict.base_value ?? null,
        autoResolvedBranchId: null,
      })
      .returning()
      .then(firstOrThrow);

    const branchRows = await tx
      .insert(DocumentConflictBranches)
      .values(
        conflict.branches.map((b) => ({
          conflictId: conflictRow.id,
          commitId: b.side === 'ours' ? oursCommitDbId : theirsCommitDbId,
          value: b.value,
        })),
      )
      .returning();

    const autoIdx = conflict.branches.findIndex((b) => b.side === conflict.auto_resolved);
    const autoBranchRow = branchRows[autoIdx];
    if (!autoBranchRow) {
      throw new TypieError({ code: 'merge_invalid_auto_resolved' });
    }

    await tx.update(DocumentConflicts).set({ autoResolvedBranchId: autoBranchRow.id }).where(eq(DocumentConflicts.id, conflictRow.id));
  }
}

async function updateHead(tx: Transaction, documentId: string, newHeadDbId: string, newHeadRootObjectId?: string): Promise<Set<string>> {
  await tx.update(Documents).set({ headCommitId: newHeadDbId }).where(eq(Documents.id, documentId));

  let rootObjectId = newHeadRootObjectId;
  if (rootObjectId === undefined) {
    const newHead = await tx
      .select({ rootObjectId: DocumentCommits.rootObjectId })
      .from(DocumentCommits)
      .where(eq(DocumentCommits.id, newHeadDbId))
      .then(firstOrThrow);
    rootObjectId = newHead.rootObjectId;
  }

  const allObjects = await walkReachableObjects(tx, rootObjectId);
  const rootObj = allObjects.find((o) => o.id === rootObjectId);
  if (!rootObj) throw new Error('root object missing after walk');

  const doc = await wasm.reconstruct_doc_from_objects(
    rootObj.hash,
    allObjects.map((o) => ({ hash: o.hash, content: o.content as ObjectContent })),
  );
  const text = await wasm.extract_text(doc);
  const characterCount = countCharacters(text);

  const imageIds: string[] = [];
  const fileIds: string[] = [];
  for (const obj of allObjects) {
    const node = (obj.content as { node?: { type?: string; id?: string } } | null)?.node;
    if (node?.type === 'image' && node.id) imageIds.push(node.id);
    else if (node?.type === 'file' && node.id) fileIds.push(node.id);
  }
  const blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);

  await tx
    .update(DocumentHeadContents)
    .set({ json: doc, text, characterCount, blobSize, updatedAt: dayjs() })
    .where(eq(DocumentHeadContents.documentId, documentId));

  const hashes = allObjects.map((o) => o.hash);
  await redis.set(`sync:walk-hashes:${rootObjectId}`, JSON.stringify(hashes), 'EX', 60 * 60 * 24 * 7);
  return new Set(hashes);
}

async function collectDelta(
  tx: Transaction,
  documentId: string,
  oldHeadId: string | null,
  newHeadId: string,
): Promise<{ deltaCommits: (typeof DocumentCommits.$inferSelect)[]; deltaObjects: (typeof DocumentObjects.$inferSelect)[] }> {
  const newAncestors = await collectAncestorIds(tx, newHeadId);
  const oldAncestors = oldHeadId ? await collectAncestorIds(tx, oldHeadId) : new Set<string>();

  const deltaCommitIds = [...newAncestors].filter((id) => !oldAncestors.has(id));
  const deltaCommits =
    deltaCommitIds.length > 0
      ? await tx
          .select()
          .from(DocumentCommits)
          .where(and(eq(DocumentCommits.documentId, documentId), inArray(DocumentCommits.id, deltaCommitIds)))
          .orderBy(asc(DocumentCommits.sequence))
      : [];

  const newRow = await tx
    .select({ rootObjectId: DocumentCommits.rootObjectId })
    .from(DocumentCommits)
    .where(eq(DocumentCommits.id, newHeadId))
    .then(firstOrThrow);
  const newHashes = await walkReachableHashes(tx, newRow.rootObjectId);

  let oldHashes = new Set<string>();
  if (oldHeadId) {
    const oldRow = await tx
      .select({ rootObjectId: DocumentCommits.rootObjectId })
      .from(DocumentCommits)
      .where(eq(DocumentCommits.id, oldHeadId))
      .then(firstOrThrow);
    oldHashes = await walkReachableHashes(tx, oldRow.rootObjectId);
  }

  const deltaHashes = [...newHashes].filter((h) => !oldHashes.has(h));
  const deltaObjects =
    deltaHashes.length > 0 ? await tx.select().from(DocumentObjects).where(inArray(DocumentObjects.hash, deltaHashes)) : [];

  return { deltaCommits, deltaObjects };
}
