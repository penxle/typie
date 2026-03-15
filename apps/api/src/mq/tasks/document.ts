import * as Sentry from '@sentry/node';
import dayjs from 'dayjs';
import { and, eq, gt, lt, sql } from 'drizzle-orm';
import { LoroDoc } from 'loro-crdt';
import * as R from 'remeda';
import { redis } from '#/cache.ts';
import {
  db,
  DocumentCharacterCountChanges,
  DocumentContents,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Entities,
  firstOrThrow,
} from '#/db/index.ts';
import { DocumentSyncType } from '#/enums.ts';
import { Lock } from '#/lock.ts';
import { pubsub } from '#/pubsub.ts';
import { compressZstd } from '#/utils/compression.ts';
import { extractLoroDocContents, garbageCollectLoroDoc } from '#/utils/index.ts';
import { enqueueJob } from '../index.ts';
import { defineCron, defineJob } from '../types.ts';

export const DocumentSyncCollectJob = defineJob('document:sync:collect', async (documentId: string) => {
  const lock = new Lock(`document:${documentId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  let updates: string[] = [];
  let parsedUpdates: { userId: string; data: string; retryCount?: number }[] = [];
  let updated = false;

  try {
    updates = (await redis.rpop(`document:sync:updates:${documentId}`, 5)) ?? [];
    if (updates.length === 0) {
      return;
    }

    const document = await db
      .select({
        id: Documents.id,
        snapshot: DocumentContents.snapshot,
        version: DocumentContents.version,
        characterCount: DocumentContents.characterCount,
        siteId: Entities.siteId,
      })
      .from(Documents)
      .innerJoin(DocumentContents, eq(Documents.id, DocumentContents.documentId))
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(firstOrThrow);

    parsedUpdates = updates.map((update) => JSON.parse(update) as { userId: string; data: string; retryCount?: number });

    const updatesByUserId = R.groupBy(parsedUpdates, ({ userId }) => userId);

    const doc = new LoroDoc();
    doc.import(document.snapshot);

    type Version = {
      userId: string;
      version: Uint8Array;
      order: number;
      delta: number;
    };

    let prevCharacterCount = document.characterCount;
    const versions: Version[] = [];
    let order = 0;

    for (const [userId, userUpdates] of Object.entries(updatesByUserId)) {
      const prevFrontiers = doc.oplogFrontiers();

      for (const { data: updateData } of userUpdates) {
        doc.import(Uint8Array.fromBase64(updateData));
      }

      const diff = doc.diff(prevFrontiers, doc.oplogFrontiers(), false);
      if (diff.length > 0) {
        const { characterCount: currentCharacterCount } = await extractLoroDocContents(doc);
        const delta = currentCharacterCount - prevCharacterCount;

        versions.push({
          userId,
          version: await compressZstd(doc.version().encode()),
          order: order++,
          delta,
        });

        prevCharacterCount = currentCharacterCount;
      }
    }

    const finalSnapshot = doc.export({ mode: 'snapshot' });
    const finalVersion = doc.version().encode();

    if (versions.length > 0) {
      const updatedAt = dayjs();
      const { json, text, characterCount, blobSize } = await extractLoroDocContents(doc);

      await db.transaction(async (tx) => {
        for (const version of versions) {
          const documentVersion = await tx
            .insert(DocumentVersions)
            .values({
              documentId,
              version: version.version,
              order: version.order,
            })
            .returning({ id: DocumentVersions.id })
            .then(firstOrThrow);

          await tx.insert(DocumentVersionContributors).values({
            versionId: documentVersion.id,
            userId: version.userId,
          });

          if (version.delta !== 0) {
            await tx
              .insert(DocumentCharacterCountChanges)
              .values({
                documentId,
                userId: version.userId,
                bucket: dayjs().startOf('hour'),
                additions: Math.max(version.delta, 0),
                deletions: Math.max(-version.delta, 0),
              })
              .onConflictDoUpdate({
                target: [
                  DocumentCharacterCountChanges.userId,
                  DocumentCharacterCountChanges.documentId,
                  DocumentCharacterCountChanges.bucket,
                ],
                set: {
                  additions: version.delta > 0 ? sql`${DocumentCharacterCountChanges.additions} + ${version.delta}` : undefined,
                  deletions: version.delta < 0 ? sql`${DocumentCharacterCountChanges.deletions} + ${-version.delta}` : undefined,
                },
              });
          }
        }

        await tx
          .update(DocumentContents)
          .set({
            json,
            text,
            characterCount,
            blobSize,
            snapshot: finalSnapshot,
            version: finalVersion,
            updatedAt,
          })
          .where(eq(DocumentContents.documentId, documentId));

        await tx.update(Documents).set({ updatedAt }).where(eq(Documents.id, documentId));

        lock.signal.throwIfAborted();
      });

      updated = true;
    } else {
      await db
        .update(DocumentContents)
        .set({
          snapshot: finalSnapshot,
          version: finalVersion,
        })
        .where(eq(DocumentContents.documentId, documentId));
    }
  } catch (err) {
    Sentry.captureException(err);

    if (parsedUpdates.length > 0) {
      const retriableUpdates = parsedUpdates
        .filter((update) => (update.retryCount ?? 0) < 10)
        .map((update) => JSON.stringify({ ...update, retryCount: (update.retryCount ?? 0) + 1 }));

      if (retriableUpdates.length > 0) {
        await redis.rpush(`document:sync:updates:${documentId}`, ...retriableUpdates.toReversed());
      }
    }
  } finally {
    await lock.release();
  }

  const updatesLeft = await redis.llen(`document:sync:updates:${documentId}`);
  if (updatesLeft > 0) {
    await enqueueJob('document:sync:collect', documentId);
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
      deduplication: {
        id: documentId,
        ttl: 60 * 1000,
      },
    });
  }
});

export const DocumentSyncScanCron = defineCron('document:sync:scan', '* * * * *', async () => {
  const keys = await redis.keys('document:sync:updates:*');

  await Promise.all(
    keys.map((key) =>
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      enqueueJob('document:sync:collect', key.split(':').at(-1)!),
    ),
  );
});

export const DocumentGCJob = defineJob('document:gc', async (documentId: string) => {
  const lock = new Lock(`document:${documentId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  try {
    const { snapshot } = await db
      .select({ snapshot: DocumentContents.snapshot })
      .from(DocumentContents)
      .where(eq(DocumentContents.documentId, documentId))
      .then(firstOrThrow);

    const doc = new LoroDoc();
    doc.import(snapshot);
    const prevVersion = doc.version();
    const deletedNodes = garbageCollectLoroDoc(doc);

    if (deletedNodes > 0) {
      const gcSnapshot = doc.export({ mode: 'snapshot' });
      const gcVersion = doc.version().encode();
      const gcUpdate = doc.export({ mode: 'update', from: prevVersion });

      lock.signal.throwIfAborted();

      await db
        .update(DocumentContents)
        .set({
          snapshot: gcSnapshot,
          version: gcVersion,
          compactedAt: dayjs(),
        })
        .where(eq(DocumentContents.documentId, documentId));

      pubsub.publish('document:sync', documentId, {
        target: '*',
        type: DocumentSyncType.UPDATE,
        data: gcUpdate.toBase64(),
      });
    } else {
      await db.update(DocumentContents).set({ compactedAt: dayjs() }).where(eq(DocumentContents.documentId, documentId));
    }
  } catch (err) {
    Sentry.captureException(err);
  } finally {
    await lock.release();
  }
});

export const DocumentGCScanCron = defineCron('document:gc:scan', '0 * * * *', async () => {
  const threshold = dayjs().subtract(24, 'hours');

  const contents = await db
    .select({ documentId: DocumentContents.documentId })
    .from(DocumentContents)
    .where(and(gt(DocumentContents.updatedAt, threshold), lt(DocumentContents.compactedAt, threshold)));

  await Promise.all(
    contents.map(({ documentId }) =>
      enqueueJob('document:gc', documentId, {
        delay: Math.floor(Math.random() * 50 * 60 * 1000),
        priority: 0,
      }),
    ),
  );
});
