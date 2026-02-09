import * as Sentry from '@sentry/bun';
import dayjs from 'dayjs';
import { and, asc, eq, gt, lt, lte, or, sql } from 'drizzle-orm';
import { LoroDoc, VersionVector } from 'loro-crdt';
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
import { DocumentSyncType, EntityState } from '@/enums';
import { Lock } from '@/lock';
import { pubsub } from '@/pubsub';
import { meilisearch } from '@/search';
import { extractLoroDocContents, garbageCollectLoroDoc } from '@/utils';
import { compressZstd, decompressZstd } from '@/utils/compression';
import { enqueueJob } from '../index';
import { defineCron, defineJob } from '../types';

type RetainedVersion = {
  id: string;
  createdAt: dayjs.Dayjs;
  userIds: Set<string>;
};

const computeRetainedVersions = (
  storedVersions: { id: string; createdAt: dayjs.Dayjs }[],
  contributorsByVersionId: Map<string, Set<string>>,
): RetainedVersion[] => {
  const threshold24h = dayjs().subtract(24, 'hours');
  const threshold2w = dayjs().subtract(2, 'weeks');

  type StoredVersion = (typeof storedVersions)[number];
  const olderVersions: StoredVersion[] = [];
  const recentVersions: StoredVersion[] = [];
  const latestVersions: StoredVersion[] = [];

  for (const version of storedVersions) {
    if (version.createdAt.isAfter(threshold24h)) {
      latestVersions.push(version);
    } else if (version.createdAt.isAfter(threshold2w)) {
      recentVersions.push(version);
    } else {
      olderVersions.push(version);
    }
  }

  const windowedVersions = new Map<string, RetainedVersion>();

  const processWindow = (versions: StoredVersion[], getBucket: (createdAt: dayjs.Dayjs) => string) => {
    for (let i = 0; i < versions.length; i++) {
      const version = versions[i];
      const userIds = [...(contributorsByVersionId.get(version.id) ?? [])];
      const isEdge = i === 0 || i === versions.length - 1;
      const bucket = isEdge ? version.id : getBucket(version.createdAt);

      const existing = windowedVersions.get(bucket);
      windowedVersions.set(bucket, {
        id: version.id,
        createdAt: version.createdAt,
        userIds: new Set([...(existing?.userIds ?? []), ...userIds]),
      });
    }
  };

  processWindow(olderVersions, (createdAt) =>
    createdAt
      .startOf('hour')
      .add(Math.floor(createdAt.minute() / 5) * 5, 'minutes')
      .toISOString(),
  );
  processWindow(recentVersions, (createdAt) => createdAt.startOf('minute').toISOString());
  processWindow(latestVersions, (createdAt) => createdAt.toISOString());

  return [...windowedVersions.values()].toSorted((a, b) => a.createdAt.valueOf() - b.createdAt.valueOf());
};

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
    const { siteId, entityId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id })
      .from(Documents)
      .innerJoin(Entities, eq(Documents.entityId, Entities.id))
      .where(eq(Documents.id, documentId))
      .then(firstOrThrow);

    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    pubsub.publish('site:usage:update', siteId, null);

    await enqueueJob('document:index', documentId, {
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

export const DocumentIndexJob = defineJob('document:index', async (documentId: string) => {
  const document = await db
    .select({
      id: Documents.id,
      state: Entities.state,
      siteId: Entities.siteId,
      title: Documents.title,
      subtitle: Documents.subtitle,
      text: DocumentContents.text,
    })
    .from(Documents)
    .innerJoin(DocumentContents, eq(Documents.id, DocumentContents.documentId))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(eq(Documents.id, documentId))
    .then(firstOrThrow);

  if (document.state === EntityState.ACTIVE) {
    await meilisearch.index('documents').addDocuments([
      {
        id: document.id,
        siteId: document.siteId,
        title: document.title,
        subtitle: document.subtitle,
        text: document.text,
      },
    ]);
  } else {
    await meilisearch.index('documents').deleteDocument(document.id);
  }
});

export const DocumentCompactJob = defineJob('document:compact', async (documentId: string) => {
  const lock = new Lock(`document:${documentId}`);

  const acquired = await lock.tryAcquire();
  if (!acquired) {
    return;
  }

  try {
    const storedVersions = await db
      .select({
        id: DocumentVersions.id,
        createdAt: DocumentVersions.createdAt,
      })
      .from(DocumentVersions)
      .where(eq(DocumentVersions.documentId, documentId))
      .orderBy(asc(DocumentVersions.createdAt), asc(DocumentVersions.order));

    let retainedVersions: RetainedVersion[] = [];

    if (storedVersions.length > 0) {
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

      retainedVersions = computeRetainedVersions(storedVersions, contributorsByVersionId);
    }

    const { snapshot } = await db
      .select({ snapshot: DocumentContents.snapshot })
      .from(DocumentContents)
      .where(eq(DocumentContents.documentId, documentId))
      .then(firstOrThrow);

    if (retainedVersions.length === storedVersions.length) {
      const doc = new LoroDoc();
      doc.import(snapshot);
      const prevVersion = doc.version();
      const deletedCount = garbageCollectLoroDoc(doc);

      if (deletedCount > 0) {
        const gcSnapshot = doc.export({ mode: 'snapshot' });
        const gcVersion = doc.version().encode();
        const gcUpdate = doc.export({ mode: 'update', from: prevVersion });

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

      return;
    }

    const doc = new LoroDoc();
    doc.import(snapshot);

    const currentFrontiers = doc.oplogFrontiers();

    type CompactedVersion = {
      version: Uint8Array;
      createdAt: dayjs.Dayjs;
      userIds: string[];
    };

    const compactedVersions: CompactedVersion[] = [];

    let startFrontiers = currentFrontiers;
    if (retainedVersions.length > 0) {
      const { version: firstVersion } = await db
        .select({ version: DocumentVersions.version })
        .from(DocumentVersions)
        .where(eq(DocumentVersions.id, retainedVersions[0].id))
        .then(firstOrThrow);
      startFrontiers = doc.vvToFrontiers(VersionVector.decode(await decompressZstd(firstVersion)));
    }

    const forkedDoc = doc.forkAt(startFrontiers);
    const baseline = new LoroDoc();
    baseline.import(forkedDoc.export({ mode: 'shallow-snapshot', frontiers: startFrontiers }));

    let prevFrontiers = startFrontiers;

    for (const [i, retainedVersion] of retainedVersions.entries()) {
      if (i > 0) {
        const { version } = await db
          .select({ version: DocumentVersions.version })
          .from(DocumentVersions)
          .where(eq(DocumentVersions.id, retainedVersion.id))
          .then(firstOrThrow);
        const frontiers = doc.vvToFrontiers(VersionVector.decode(await decompressZstd(version)));
        const diff = doc.diff(prevFrontiers, frontiers, false);
        prevFrontiers = frontiers;
        if (diff.length === 0) {
          continue;
        }
        baseline.applyDiff(diff);
      }

      compactedVersions.push({
        version: await compressZstd(baseline.version().encode()),
        createdAt: retainedVersion.createdAt,
        userIds: [...retainedVersion.userIds],
      });
    }

    if (retainedVersions.length > 0) {
      const finalDiff = doc.diff(prevFrontiers, currentFrontiers, false);
      baseline.applyDiff(finalDiff);
    }

    garbageCollectLoroDoc(baseline);

    const finalSnapshot = baseline.export({ mode: 'snapshot' });
    const finalVersion = baseline.version().encode();

    await db.transaction(async (tx) => {
      if (storedVersions.length > 0) {
        await tx.delete(DocumentVersions).where(eq(DocumentVersions.documentId, documentId));
      }

      for (const version of compactedVersions) {
        const documentVersion = await tx
          .insert(DocumentVersions)
          .values({
            documentId,
            version: version.version,
            createdAt: version.createdAt,
            order: 0,
          })
          .returning({ id: DocumentVersions.id })
          .then(firstOrThrow);

        if (version.userIds.length > 0) {
          await tx.insert(DocumentVersionContributors).values(
            version.userIds.map((userId) => ({
              versionId: documentVersion.id,
              userId,
            })),
          );
        }
      }

      await tx
        .update(DocumentContents)
        .set({
          snapshot: finalSnapshot,
          version: finalVersion,
          compactedAt: dayjs(),
        })
        .where(eq(DocumentContents.documentId, documentId));

      lock.signal.throwIfAborted();
    });
  } catch (err) {
    Sentry.captureException(err);
  } finally {
    await lock.release();
  }
});

export const DocumentCompactScanCron = defineCron('document:compact:scan', '0 * * * *', async () => {
  const now = dayjs();

  const threshold12h = now.subtract(12, 'hours');
  const threshold24h = now.subtract(24, 'hours');
  const threshold48h = now.subtract(48, 'hours');
  const threshold7d = now.subtract(7, 'days');

  const contents = await db
    .select({ documentId: DocumentContents.documentId })
    .from(DocumentContents)
    .where(
      or(
        and(
          lte(DocumentContents.updatedAt, threshold24h),
          gt(DocumentContents.updatedAt, threshold48h),
          lt(DocumentContents.compactedAt, threshold12h),
        ),
        and(
          lt(DocumentContents.compactedAt, threshold7d),
          lt(DocumentContents.compactedAt, sql`${DocumentContents.updatedAt} + interval '30 days'`),
        ),
      ),
    );

  await Promise.all(
    contents.map(({ documentId }) =>
      enqueueJob('document:compact', documentId, {
        delay: Math.floor(Math.random() * 50 * 60 * 1000),
        priority: 0,
      }),
    ),
  );
});
