import dayjs from 'dayjs';
import { and, asc, eq, gt, lt, lte, notInArray, or, sql } from 'drizzle-orm';
import { rapidhash } from 'rapidhash-js';
import * as R from 'remeda';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { CanvasContents, Canvases, CanvasSnapshotContributors, CanvasSnapshots, db, Entities, firstOrThrow } from '@/db';
import { EntityState } from '@/enums';
import { pubsub } from '@/pubsub';
import { meilisearch } from '@/search';
import { compressZstd, decompressZstd } from '@/utils/compression';
import { enqueueJob } from '../index';
import { defineCron, defineJob } from '../types';
import type { CanvasShape } from '@/db/schemas/json';

export const CanvasSyncCollectJob = defineJob('canvas:sync:collect', async (canvasId: string) => {
  const updates = await redis.rpop(`canvas:sync:updates:${canvasId}`, 5);
  if (updates === null || updates.length === 0) {
    return;
  }

  let snapshotUpdated = false;

  try {
    await db.transaction(async (tx) => {
      const hash = BigInt(rapidhash(canvasId)) % BigInt('9223372036854775807');
      await tx.execute(sql`SELECT pg_advisory_xact_lock(${hash})`);

      const canvas = await tx
        .select({
          id: Canvases.id,
          update: CanvasContents.update,
          siteId: Entities.siteId,
        })
        .from(Canvases)
        .innerJoin(CanvasContents, eq(Canvases.id, CanvasContents.canvasId))
        .innerJoin(Entities, eq(Canvases.entityId, Entities.id))
        .where(eq(Canvases.id, canvasId))
        .then(firstOrThrow);

      const pendingUpdates = R.groupBy(
        updates.map((update) => JSON.parse(update) as { userId: string; data: string }),
        ({ userId }) => userId,
      );

      const doc = new Y.Doc({ gc: false });
      Y.applyUpdateV2(doc, canvas.update);

      let order = 0;

      for (const [userId, data] of Object.entries(pendingUpdates)) {
        const prevSnapshot = Y.snapshot(doc);
        const update = Y.mergeUpdatesV2(data.map(({ data }) => Uint8Array.fromBase64(data)));

        Y.applyUpdateV2(doc, update);
        const snapshot = Y.snapshot(doc);

        if (!Y.equalSnapshots(prevSnapshot, snapshot)) {
          snapshotUpdated = true;

          const snapshotData = Y.encodeSnapshotV2(snapshot);
          const compressedSnapshot = await compressZstd(snapshotData);

          const canvasSnapshot = await tx
            .insert(CanvasSnapshots)
            .values({
              canvasId,
              snapshot: compressedSnapshot,
              order: order++,
            })
            .returning({ id: CanvasSnapshots.id })
            .then(firstOrThrow);

          await tx.insert(CanvasSnapshotContributors).values({
            snapshotId: canvasSnapshot.id,
            userId,
          });
        }
      }

      await tx
        .update(CanvasContents)
        .set({
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        })
        .where(eq(CanvasContents.canvasId, canvasId));

      if (snapshotUpdated) {
        const map = doc.getMap('attrs');
        const title = (map.get('title') as string) || null;

        const fragment = doc.getXmlFragment('shapes');
        const shapes: CanvasShape[] = [];
        fragment.forEach((element) => {
          if (element instanceof Y.XmlElement) {
            const type = element.nodeName;
            const attrs: Record<string, unknown> = {};
            for (const [key, value] of Object.entries(element.getAttributes())) {
              if (value !== undefined && value !== null) {
                attrs[key] = JSON.parse(value as string);
              }
            }

            shapes.push({ type, attrs });
          }
        });

        const updatedAt = dayjs();

        await tx
          .update(Canvases)
          .set({
            title,
            updatedAt,
          })
          .where(eq(Canvases.id, canvasId));

        await tx
          .update(CanvasContents)
          .set({
            shapes,
            updatedAt,
          })
          .where(eq(CanvasContents.canvasId, canvasId));
      }
    });
  } catch {
    await redis.rpush(`canvas:sync:updates:${canvasId}`, ...updates);
  }

  const updatesLeft = await redis.llen(`canvas:sync:updates:${canvasId}`);
  if (updatesLeft > 0) {
    await enqueueJob('canvas:sync:collect', canvasId);
  }

  if (snapshotUpdated) {
    const { siteId, entityId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id })
      .from(Canvases)
      .innerJoin(Entities, eq(Canvases.entityId, Entities.id))
      .where(eq(Canvases.id, canvasId))
      .then(firstOrThrow);

    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    pubsub.publish('site:usage:update', siteId, null);

    await enqueueJob('canvas:index', canvasId, {
      deduplication: {
        id: canvasId,
        ttl: 60 * 1000,
      },
    });
  }
});

export const CanvasSyncScanCron = defineCron('canvas:sync:scan', '* * * * *', async () => {
  const keys = await redis.keys('canvas:sync:updates:*');

  await Promise.all(
    keys.map((key) =>
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      enqueueJob('canvas:sync:collect', key.split(':').at(-1)!),
    ),
  );
});

type Snapshot = { id: string; createdAt: dayjs.Dayjs; userIds: Set<string> };

export const CanvasCompactJob = defineJob('canvas:compact', async (canvasId: string) => {
  await db.transaction(async (tx) => {
    const hash = BigInt(rapidhash(canvasId)) % BigInt('9223372036854775807');
    await tx.execute(sql`SELECT pg_advisory_xact_lock(${hash})`);

    const snapshots = await tx
      .select({ id: CanvasSnapshots.id, createdAt: CanvasSnapshots.createdAt })
      .from(CanvasSnapshots)
      .where(eq(CanvasSnapshots.canvasId, canvasId))
      .orderBy(asc(CanvasSnapshots.createdAt), asc(CanvasSnapshots.order));

    if (snapshots.length === 0) {
      await tx.update(CanvasContents).set({ compactedAt: dayjs() }).where(eq(CanvasContents.canvasId, canvasId));
      return;
    }

    const contributors = await tx
      .select({ snapshotId: CanvasSnapshotContributors.snapshotId, userId: CanvasSnapshotContributors.userId })
      .from(CanvasSnapshotContributors)
      .innerJoin(CanvasSnapshots, eq(CanvasSnapshotContributors.snapshotId, CanvasSnapshots.id))
      .where(eq(CanvasSnapshots.canvasId, canvasId));

    const contributorsBySnapshotId = new Map<string, Set<string>>();
    for (const contributor of contributors) {
      const userIds = contributorsBySnapshotId.get(contributor.snapshotId) ?? new Set();
      userIds.add(contributor.userId);
      contributorsBySnapshotId.set(contributor.snapshotId, userIds);
    }

    const threshold24h = dayjs().subtract(24, 'hours');
    const threshold2w = dayjs().subtract(2, 'weeks');
    const windowedSnapshots = new Map<string, Snapshot>();

    for (const snapshot of snapshots) {
      const userIds = [...(contributorsBySnapshotId.get(snapshot.id) ?? [])];

      const window = snapshot.createdAt.isAfter(threshold24h)
        ? snapshot.createdAt.toISOString()
        : snapshot.createdAt.isAfter(threshold2w)
          ? snapshot.createdAt.startOf('minute').toISOString()
          : snapshot.createdAt.startOf('hour').toISOString();

      const windowedSnapshot = windowedSnapshots.get(window);

      if (windowedSnapshot) {
        windowedSnapshots.set(window, {
          id: snapshot.id,
          createdAt: snapshot.createdAt,
          userIds: new Set([...windowedSnapshot.userIds, ...userIds]),
        });
      } else {
        windowedSnapshots.set(window, {
          id: snapshot.id,
          createdAt: snapshot.createdAt,
          userIds: new Set(userIds),
        });
      }
    }

    const retainedSnapshots = [...windowedSnapshots.values()].sort((a, b) => a.createdAt.valueOf() - b.createdAt.valueOf());

    if (retainedSnapshots.length === snapshots.length) {
      await tx.update(CanvasContents).set({ compactedAt: dayjs() }).where(eq(CanvasContents.canvasId, canvasId));
      return;
    }

    const content = await tx
      .select({ update: CanvasContents.update })
      .from(CanvasContents)
      .where(eq(CanvasContents.canvasId, canvasId))
      .then(firstOrThrow);

    const oldDoc = new Y.Doc({ gc: false });
    Y.applyUpdateV2(oldDoc, content.update);

    await tx.delete(CanvasSnapshots).where(
      and(
        eq(CanvasSnapshots.canvasId, canvasId),
        notInArray(
          CanvasSnapshots.id,
          retainedSnapshots.map(({ id }) => id),
        ),
      ),
    );

    const newDoc = new Y.Doc({ gc: false });
    let index = 0;

    for (const snapshot of retainedSnapshots) {
      const { snapshot: snapshotData } = await tx
        .delete(CanvasSnapshots)
        .where(eq(CanvasSnapshots.id, snapshot.id))
        .returning({ snapshot: CanvasSnapshots.snapshot })
        .then(firstOrThrow);

      let snapshotDoc;
      try {
        const decompressedSnapshot = await decompressZstd(snapshotData);
        snapshotDoc = Y.createDocFromSnapshot(oldDoc, Y.decodeSnapshotV2(decompressedSnapshot));
      } catch {
        continue;
      }

      if (index === 0) {
        Y.applyUpdateV2(newDoc, Y.encodeStateAsUpdateV2(snapshotDoc));
      } else {
        const newStateVector = Y.encodeStateVector(newDoc);
        const snapshotStateVector = Y.encodeStateVector(snapshotDoc);

        const missingUpdate = Y.encodeStateAsUpdateV2(newDoc, snapshotStateVector);

        const undoManager = new Y.UndoManager(snapshotDoc, { trackedOrigins: new Set(['snapshot']) });
        Y.applyUpdateV2(snapshotDoc, missingUpdate, 'snapshot');
        undoManager.undo();

        const revertUpdate = Y.encodeStateAsUpdateV2(snapshotDoc, newStateVector);
        Y.applyUpdateV2(newDoc, revertUpdate);
      }

      const newSnapshotData = Y.encodeSnapshotV2(Y.snapshot(newDoc));
      const compressedNewSnapshot = await compressZstd(newSnapshotData);

      const canvasSnapshot = await tx
        .insert(CanvasSnapshots)
        .values({
          canvasId,
          snapshot: compressedNewSnapshot,
          createdAt: snapshot.createdAt,
          order: 0,
        })
        .returning({ id: CanvasSnapshots.id })
        .then(firstOrThrow);

      if (snapshot.userIds.size > 0) {
        await tx.insert(CanvasSnapshotContributors).values(
          [...snapshot.userIds].map((userId) => ({
            snapshotId: canvasSnapshot.id,
            userId,
          })),
        );
      }

      index++;
    }

    const beforeSnapshot = Y.snapshot(newDoc);

    const newStateVector = Y.encodeStateVector(newDoc);
    const oldStateVector = Y.encodeStateVector(oldDoc);

    const missingUpdate = Y.encodeStateAsUpdateV2(newDoc, oldStateVector);

    const undoManager = new Y.UndoManager(oldDoc, { trackedOrigins: new Set(['snapshot']) });
    Y.applyUpdateV2(oldDoc, missingUpdate, 'snapshot');
    undoManager.undo();

    const revertUpdate = Y.encodeStateAsUpdateV2(oldDoc, newStateVector);
    Y.applyUpdateV2(newDoc, revertUpdate);

    const afterSnapshot = Y.snapshot(newDoc);

    if (!Y.equalSnapshots(beforeSnapshot, afterSnapshot)) {
      const finalSnapshotData = Y.encodeSnapshotV2(afterSnapshot);
      const compressedFinalSnapshot = await compressZstd(finalSnapshotData);

      await tx.insert(CanvasSnapshots).values({
        canvasId,
        snapshot: compressedFinalSnapshot,
        order: 0,
      });
    }

    await tx
      .update(CanvasContents)
      .set({
        update: Y.encodeStateAsUpdateV2(newDoc),
        vector: Y.encodeStateVector(newDoc),
        compactedAt: dayjs(),
      })
      .where(eq(CanvasContents.canvasId, canvasId));
  });
});

export const CanvasIndexJob = defineJob('canvas:index', async (canvasId: string) => {
  const canvas = await db
    .select({
      id: Canvases.id,
      state: Entities.state,
      siteId: Entities.siteId,
      title: Canvases.title,
    })
    .from(Canvases)
    .innerJoin(Entities, eq(Canvases.entityId, Entities.id))
    .where(eq(Canvases.id, canvasId))
    .then(firstOrThrow);

  if (canvas.state === EntityState.ACTIVE) {
    await meilisearch.index('canvases').addDocuments([
      {
        id: canvas.id,
        siteId: canvas.siteId,
        title: canvas.title,
      },
    ]);
  } else {
    await meilisearch.index('canvases').deleteDocument(canvas.id);
  }
});

export const CanvasCompactScanCron = defineCron('canvas:compact:scan', '0 * * * *', async () => {
  const now = dayjs();

  const threshold1h = now.subtract(1, 'hour');
  const threshold6h = now.subtract(6, 'hours');
  const threshold24h = now.subtract(24, 'hours');

  const threshold1d = now.subtract(1, 'day');
  const threshold2d = now.subtract(2, 'days');
  const threshold14d = now.subtract(14, 'days');

  const canvases = await db
    .select({ canvasId: CanvasContents.canvasId })
    .from(CanvasContents)
    .where(
      or(
        and(
          lte(CanvasContents.updatedAt, threshold1h),
          gt(CanvasContents.updatedAt, threshold24h),
          lt(CanvasContents.compactedAt, threshold6h),
        ),
        and(
          lte(CanvasContents.updatedAt, threshold1d),
          gt(CanvasContents.updatedAt, threshold14d),
          lt(CanvasContents.compactedAt, threshold2d),
        ),
      ),
    );

  await Promise.all(
    canvases.map(({ canvasId }) =>
      enqueueJob('canvas:compact', canvasId, {
        delay: Math.floor(Math.random() * 50 * 60 * 1000),
        priority: 0,
      }),
    ),
  );
});
