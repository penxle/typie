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
import { calculateBlobSizeFromAssetIds, countCharacters, extractAssetIdsFromPlainDoc } from '#/utils/entity.ts';
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

  try {
    const updates = await redis.lrange(`document:changesets:pending:${documentId}`, -5, -1);
    if (updates.length === 0) {
      return;
    }

    const parsedUpdates: { userId: string; deviceId: string; changesets: string }[] = [...updates].toReversed().map((u) => JSON.parse(u));
    const newBundles = parsedUpdates.map((p) => Uint8Array.fromBase64(p.changesets));

    const { graph: existing, characterCount: baseCharacterCount } = await db
      .select({ graph: DocumentStates.graph, characterCount: DocumentStates.characterCount })
      .from(DocumentStates)
      .where(eq(DocumentStates.documentId, documentId))
      .then(firstOrThrow);

    const failed: { userId: string; deviceId: string; payload: Uint8Array; error: string }[] = [];
    const perUserDelta = new Map<string, number>();
    const contributorIds = new Set<string>();

    const result = await wasm.use((host) => {
      let merged = existing;
      let mergedChanged = false;
      let prevCharacterCount = baseCharacterCount;

      for (const [i, bundle] of newBundles.entries()) {
        try {
          const candidate = host.apply(merged, bundle);
          const candidatePlain = host.to_plain(candidate);
          host.verify_plain(candidatePlain);

          const candidateCharacterCount = countCharacters(host.extract_text(candidatePlain));
          const { userId } = parsedUpdates[i];
          perUserDelta.set(userId, (perUserDelta.get(userId) ?? 0) + (candidateCharacterCount - prevCharacterCount));
          prevCharacterCount = candidateCharacterCount;

          merged = candidate;
          mergedChanged = true;
          contributorIds.add(userId);
        } catch (err) {
          failed.push({ userId: parsedUpdates[i].userId, deviceId: parsedUpdates[i].deviceId, payload: bundle, error: String(err) });
        }
      }

      if (!mergedChanged) {
        return null;
      }

      const plain = host.to_plain(merged);
      const text = host.extract_text(plain);
      const characterCount = countCharacters(text);
      return { graph: merged, plain, text, characterCount, heads: host.heads(merged) };
    });

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

    await redis.ltrim(`document:changesets:pending:${documentId}`, 0, -(updates.length + 1));
  } catch (err) {
    Sentry.captureException(err);
    throw err;
  } finally {
    await lock.release();
  }

  const left = await redis.llen(`document:changesets:pending:${documentId}`);
  if (left > 0) {
    await enqueueJob('document:changesets:collect', documentId);
  }

  if (updated) {
    const { siteId, entityId, userId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id, userId: Entities.userId })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(firstOrThrow);

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
  const keys = await redis.keys('document:changesets:pending:*');

  await Promise.all(
    keys.map((key) =>
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      enqueueJob('document:changesets:collect', key.split(':').at(-1)!),
    ),
  );
});
