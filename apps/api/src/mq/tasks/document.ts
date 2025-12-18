import * as Sentry from '@sentry/bun';
import dayjs from 'dayjs';
import { asc, eq, lt, sql } from 'drizzle-orm';
import { LoroDoc } from 'loro-crdt';
import * as R from 'remeda';
import { redis } from '@/cache';
import {
  db,
  DocumentCharacterCountChanges,
  DocumentContents,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Entities,
  firstOrThrow,
} from '@/db';
import { Lock } from '@/lock';
import { pubsub } from '@/pubsub';
import { extractLoroDocContents } from '@/utils';
import { enqueueJob } from '../index';
import { defineCron, defineJob } from '../types';

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

    const pendingUpdates = R.groupBy(parsedUpdates, ({ userId }) => userId);

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

    for (const [userId, data] of Object.entries(pendingUpdates)) {
      const prevVersionEncoded = doc.version().encode();

      for (const { data: updateData } of data) {
        doc.import(Uint8Array.fromBase64(updateData));
      }

      const newVersionEncoded = doc.version().encode();

      const versionsEqual =
        prevVersionEncoded.length === newVersionEncoded.length && prevVersionEncoded.every((v, i) => v === newVersionEncoded[i]);

      if (!versionsEqual) {
        const { characterCount: currentCharacterCount } = extractLoroDocContents(doc);
        const delta = currentCharacterCount - prevCharacterCount;

        versions.push({
          userId,
          version: newVersionEncoded,
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
      const { json, text, characterCount, blobSize } = extractLoroDocContents(doc);

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
    const { siteId, entityId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(firstOrThrow);

    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    pubsub.publish('site:usage:update', siteId, null);
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

export const DocumentCompactJob = defineJob('document:compact', async (documentId: string) => {
  const lock = new Lock(`document:${documentId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  try {
    type Version = { id: string; createdAt: dayjs.Dayjs; userIds: Set<string> };

    const storedVersions = await db
      .select({
        id: DocumentVersions.id,
        createdAt: DocumentVersions.createdAt,
      })
      .from(DocumentVersions)
      .where(eq(DocumentVersions.documentId, documentId))
      .orderBy(asc(DocumentVersions.createdAt), asc(DocumentVersions.order));

    if (storedVersions.length === 0) {
      await db.update(DocumentContents).set({ compactedAt: dayjs() }).where(eq(DocumentContents.documentId, documentId));
      return;
    }

    const contributors = await db
      .select({
        versionId: DocumentVersionContributors.versionId,
        userId: DocumentVersionContributors.userId,
      })
      .from(DocumentVersionContributors)
      .innerJoin(DocumentVersions, eq(DocumentVersionContributors.versionId, DocumentVersions.id))
      .where(eq(DocumentVersions.documentId, documentId));

    const contributorsByVersionId = new Map<string, Set<string>>();
    for (const contributor of contributors) {
      const userIds = contributorsByVersionId.get(contributor.versionId) ?? new Set();
      userIds.add(contributor.userId);
      contributorsByVersionId.set(contributor.versionId, userIds);
    }

    const threshold24h = dayjs().subtract(24, 'hours');
    const threshold2w = dayjs().subtract(2, 'weeks');
    const windowedVersions = new Map<string, Version>();

    for (const version of storedVersions) {
      const userIds = [...(contributorsByVersionId.get(version.id) ?? [])];

      const window = version.createdAt.isAfter(threshold24h)
        ? version.createdAt.toISOString()
        : version.createdAt.isAfter(threshold2w)
          ? version.createdAt.startOf('minute').toISOString()
          : version.createdAt.startOf('hour').toISOString();

      const windowedVersion = windowedVersions.get(window);

      if (windowedVersion) {
        windowedVersions.set(window, {
          id: version.id,
          createdAt: version.createdAt,
          userIds: new Set([...windowedVersion.userIds, ...userIds]),
        });
      } else {
        windowedVersions.set(window, {
          id: version.id,
          createdAt: version.createdAt,
          userIds: new Set(userIds),
        });
      }
    }

    const retainedVersions = [...windowedVersions.values()].toSorted((a, b) => a.createdAt.valueOf() - b.createdAt.valueOf());

    if (retainedVersions.length === storedVersions.length) {
      await db.update(DocumentContents).set({ compactedAt: dayjs() }).where(eq(DocumentContents.documentId, documentId));
      return;
    }

    const retainedVersionIds = new Set(retainedVersions.map((v) => v.id));
    const versionsToDelete = storedVersions.filter((v) => !retainedVersionIds.has(v.id));

    await db.transaction(async (tx) => {
      for (const version of versionsToDelete) {
        await tx.delete(DocumentVersions).where(eq(DocumentVersions.id, version.id));
      }

      await tx.update(DocumentContents).set({ compactedAt: dayjs() }).where(eq(DocumentContents.documentId, documentId));

      lock.signal.throwIfAborted();
    });
  } catch (err) {
    Sentry.captureException(err);
  } finally {
    await lock.release();
  }
});

export const DocumentCompactScanCron = defineCron('document:compact:scan', '0 * * * *', async () => {
  const contents = await db
    .select({ documentId: DocumentContents.documentId })
    .from(DocumentContents)
    .where(lt(DocumentContents.compactedAt, dayjs().subtract(1, 'hour')));

  await Promise.all(contents.map(({ documentId }) => enqueueJob('document:compact', documentId)));
});
