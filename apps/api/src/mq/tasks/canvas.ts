import dayjs from 'dayjs';
import { and, asc, eq, lt, notInArray, sql } from 'drizzle-orm';
import { rapidhash } from 'rapidhash-js';
import * as R from 'remeda';
import { base64 } from 'rfc4648';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { CanvasContents, Canvases, CanvasSnapshotContributors, CanvasSnapshots, db, Entities, firstOrThrow } from '@/db';
import { pubsub } from '@/pubsub';
import { queue } from '../bullmq';
import { enqueueJob } from '../index';
import { defineCron, defineJob } from '../types';
import type { CanvasShape } from '@/db/schemas/json';

export const CanvasSyncCollectJob = defineJob('canvas:sync:collect', async (canvasId: string) => {
  const updatesAvailable = await redis.scard(`canvas:sync:updates:${canvasId}`);
  if (updatesAvailable === 0) {
    return;
  }

  let snapshotUpdated = false;

  const updates = await redis.smembers(`canvas:sync:updates:${canvasId}`);
  if (updates.length === 0) {
    return;
  }

  await db.transaction(async (tx) => {
    const hash = rapidhash(canvasId);
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
      const update = Y.mergeUpdatesV2(data.map(({ data }) => base64.parse(data)));

      Y.applyUpdateV2(doc, update);
      const snapshot = Y.snapshot(doc);

      if (!Y.equalSnapshots(prevSnapshot, snapshot)) {
        snapshotUpdated = true;

        const canvasSnapshot = await tx
          .insert(CanvasSnapshots)
          .values({
            canvasId,
            snapshot: Y.encodeSnapshotV2(snapshot),
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

  await redis.srem(`canvas:sync:updates:${canvasId}`, ...updates);

  const updatesLeft = await redis.scard(`canvas:sync:updates:${canvasId}`);
  if (updatesLeft > 0) {
    await queue.removeDeduplicationKey(`canvas:sync:collect:${canvasId}`);
    await enqueueJob('canvas:sync:collect', canvasId, {
      deduplication: {
        id: `canvas:sync:collect:${canvasId}`,
      },
    });
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
  }
});

type Snapshot = { id: string; createdAt: dayjs.Dayjs; userIds: Set<string> };

export const CanvasCompactJob = defineJob('canvas:compact', async (canvasId: string) => {
  await db.transaction(async (tx) => {
    const hash = rapidhash(canvasId);
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
        snapshotDoc = Y.createDocFromSnapshot(oldDoc, Y.decodeSnapshotV2(snapshotData));
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

      const canvasSnapshot = await tx
        .insert(CanvasSnapshots)
        .values({
          canvasId,
          snapshot: Y.encodeSnapshotV2(Y.snapshot(newDoc)),
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
      await tx.insert(CanvasSnapshots).values({
        canvasId,
        snapshot: Y.encodeSnapshotV2(afterSnapshot),
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

export const CanvasCompactScanCron = defineCron('canvas:compact:scan', '0 * * * *', async () => {
  const threshold = dayjs().subtract(24, 'hours');

  const canvases = await db
    .select({ canvasId: CanvasContents.canvasId })
    .from(CanvasContents)
    .where(and(lt(CanvasContents.updatedAt, threshold), lt(CanvasContents.compactedAt, CanvasContents.updatedAt)));

  await Promise.all(
    canvases.map(({ canvasId }) =>
      enqueueJob('canvas:compact', canvasId, {
        delay: Math.random() * 10 * 60 * 1000,
        priority: 1,
      }),
    ),
  );
});
