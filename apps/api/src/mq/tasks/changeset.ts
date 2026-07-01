import * as Sentry from '@sentry/node';
import dayjs from 'dayjs';
import { eq, sql } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentChangesetsDeadLetter,
  DocumentCharacterCountChanges,
  DocumentHeadContributors,
  DocumentHeads,
  Documents,
  DocumentStates,
  Entities,
  firstOrThrow,
} from '#/db/index.ts';
import { Lock } from '#/lock.ts';
import { pubsub } from '#/pubsub.ts';
import { collectedKey, durableKey, packLengthPrefixed, readStreamBatch, trimStream } from '#/utils/changeset.ts';
import { calculateBlobSizeFromAssetIds, extractAssetIdsFromPlainDoc } from '#/utils/entity.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { enqueueJob } from '../index.ts';
import { defineCron, defineJob } from '../types.ts';
import type { Dayjs } from 'dayjs';

const HEAD_BUCKET_SECONDS = 10 * 60;

const headBucket = (ts: Dayjs): Dayjs => dayjs.unix(Math.floor(ts.unix() / HEAD_BUCKET_SECONDS) * HEAD_BUCKET_SECONDS);

export const DocumentChangesetsCollectJob = defineJob('document:changesets:collect', async (documentId: string) => {
  const lock = new Lock(`document:changesets:${documentId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  let updated = false;
  let persistedHeads: Uint8Array | null = null;

  try {
    const collected = await redis.get(collectedKey(documentId));
    const entries = await readStreamBatch(documentId, collected, 5);
    if (entries.length === 0) {
      return;
    }

    const { graph: existing } = await db
      .select({ graph: DocumentStates.graph })
      .from(DocumentStates)
      .where(eq(DocumentStates.documentId, documentId))
      .then(firstOrThrow);

    const failed: { userId: string; deviceId: string; payload: Uint8Array; error: string }[] = [];
    const perUserDelta = new Map<string, number>();
    const contributorIds = new Set<string>();

    // One State build for the whole batch (amortized) with per-bundle character
    // counts — replaces the per-entry `from_changesets` rebuild that made collect
    // `O(tail × build)`.
    const packed = packLengthPrefixed(entries.map((e) => e.changeset));
    const fold = await wasm.use((host) => host.collect_fold(existing, packed));

    let mergedChanged = false;
    let prevCharacterCount = fold.base_char_count;
    for (const [i, entry] of entries.entries()) {
      const applied = fold.applied[i];
      const charCount = fold.char_counts[i];
      if (applied) {
        perUserDelta.set(entry.userId, (perUserDelta.get(entry.userId) ?? 0) + (charCount - prevCharacterCount));
        prevCharacterCount = charCount;
        contributorIds.add(entry.userId);
        mergedChanged = true;
      } else {
        failed.push({ userId: entry.userId, deviceId: entry.deviceId, payload: entry.changeset, error: 'changeset apply failed' });
      }
    }

    const result = mergedChanged
      ? {
          graph: fold.graph,
          plain: fold.plain,
          text: fold.text,
          characterCount: fold.char_counts.at(-1) ?? fold.base_char_count,
          heads: fold.heads,
        }
      : null;

    if (result || failed.length > 0) {
      let imageIds: string[] = [];
      let fileIds: string[] = [];
      let blobSize = 0;
      let characterCount = 0;

      if (result) {
        const ids = extractAssetIdsFromPlainDoc(result.plain);
        imageIds = ids.imageIds;
        fileIds = ids.fileIds;
        blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);
        characterCount = result.characterCount;
      }

      const updatedAt = dayjs();

      await db.transaction(async (tx) => {
        lock.signal.throwIfAborted();

        if (result) {
          await tx
            .update(DocumentStates)
            .set({ graph: result.graph, json: result.plain, text: result.text, characterCount, blobSize, updatedAt })
            .where(eq(DocumentStates.documentId, documentId));
          await tx.update(Documents).set({ updatedAt }).where(eq(Documents.id, documentId));
          updated = true;
          persistedHeads = result.heads;

          for (const [userId, net] of perUserDelta) {
            if (net === 0) {
              continue;
            }

            await tx
              .insert(DocumentCharacterCountChanges)
              .values({
                documentId,
                userId,
                bucket: updatedAt.startOf('hour'),
                additions: Math.max(net, 0),
                deletions: Math.max(-net, 0),
              })
              .onConflictDoUpdate({
                target: [
                  DocumentCharacterCountChanges.userId,
                  DocumentCharacterCountChanges.documentId,
                  DocumentCharacterCountChanges.bucket,
                ],
                set: {
                  additions: net > 0 ? sql`${DocumentCharacterCountChanges.additions} + ${net}` : undefined,
                  deletions: net < 0 ? sql`${DocumentCharacterCountChanges.deletions} + ${-net}` : undefined,
                },
              });
          }

          const headBucketAt = headBucket(updatedAt);
          const headRow = await tx
            .insert(DocumentHeads)
            .values({ documentId, bucket: headBucketAt, heads: result.heads, characterCount })
            .onConflictDoUpdate({
              target: [DocumentHeads.documentId, DocumentHeads.bucket],
              set: { heads: result.heads, characterCount, updatedAt },
            })
            .returning({ id: DocumentHeads.id })
            .then(firstOrThrow);

          for (const userId of contributorIds) {
            await tx
              .insert(DocumentHeadContributors)
              .values({ headId: headRow.id, userId })
              .onConflictDoNothing({ target: [DocumentHeadContributors.headId, DocumentHeadContributors.userId] });
          }
        }

        if (failed.length > 0) {
          await tx.insert(DocumentChangesetsDeadLetter).values(
            failed.map((f) => ({
              documentId,
              payload: f.payload,
              userId: f.userId,
              deviceId: f.deviceId,
              errorMessage: f.error,
            })),
          );
        }
      });

      if (failed.length > 0) {
        for (const f of failed) {
          Sentry.captureMessage(`changeset dead-lettered: documentId=${documentId} userId=${f.userId} error=${f.error}`);
        }
      }
    }

    // Advance the collected cursor past this batch (applied + dead-lettered),
    // record the durable snapshot heads, then trim history behind the cursor.
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- entries is non-empty (guarded above)
    const lastSeq = entries.at(-1)!.seq;
    await redis.set(collectedKey(documentId), lastSeq);
    if (result) {
      await redis.set(durableKey(documentId), result.heads.toBase64());
    }
    await trimStream(documentId, lastSeq);
  } catch (err) {
    Sentry.captureException(err);
    throw err;
  } finally {
    await lock.release();
  }

  const collectedNow = await redis.get(collectedKey(documentId));
  const remaining = await readStreamBatch(documentId, collectedNow, 1);
  if (remaining.length > 0) {
    await enqueueJob('document:changesets:collect', documentId);
  }

  if (updated) {
    const { siteId, entityId, userId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id, userId: Entities.userId })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(firstOrThrow);

    pubsub.publish('document:changesets', documentId, {
      target: '*',
      seq: '',
      changesets: [],
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      heads: persistedHeads!.toBase64(),
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      durableHeads: persistedHeads!.toBase64(),
    });
    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    pubsub.publish('user:usage:update', userId, null);

    await enqueueJob('search:index:document', documentId, {
      deduplication: { id: documentId, ttl: 60 * 1000 },
    });

    await enqueueJob('document:preview:invalidate', documentId, {
      deduplication: { id: documentId, ttl: 60 * 60 * 1000 },
    });
  }
});

export const DocumentChangesetsScanCron = defineCron('document:changesets:scan', '* * * * *', async () => {
  const keys = await redis.keys('document:changesets:stream:*');

  await Promise.all(
    keys.map((key) =>
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      enqueueJob('document:changesets:collect', key.split(':').at(-1)!),
    ),
  );
});
