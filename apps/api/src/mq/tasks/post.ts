import { findChildren } from '@tiptap/core';
import dayjs from 'dayjs';
import { and, asc, eq, lt, notInArray, sql } from 'drizzle-orm';
import * as R from 'remeda';
import { base64 } from 'rfc4648';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis, redlock } from '@/cache';
import {
  db,
  Entities,
  firstOrThrow,
  PostAnchors,
  PostCharacterCountChanges,
  PostContents,
  Posts,
  PostSnapshotContributors,
  PostSnapshots,
} from '@/db';
import { EntityState } from '@/enums';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { meilisearch } from '@/search';
import { makeText } from '@/utils';
import { queue } from '../bullmq';
import { enqueueJob } from '../index';
import { defineCron, defineJob } from '../types';
import type { Node } from '@tiptap/pm/model';

const getCharacterCount = (text: string) => {
  return [...text.replaceAll(/\s+/g, ' ').trim()].length;
};

const getBlobSize = (node: Node) => {
  const sizes = findChildren(node, (node) => node.type.name === 'file' || node.type.name === 'image').map(
    ({ node }) => Number(node.attrs.size) || 0,
  );
  return sizes.reduce((acc, size) => acc + size, 0);
};

export const PostSyncCollectJob = defineJob('post:sync:collect', async (postId: string) => {
  const updatesAvailable = await redis.scard(`post:sync:updates:${postId}`);
  if (updatesAvailable === 0) {
    return;
  }

  let snapshotUpdated = false;

  await redlock.using([`{lock}:post:${postId}`], 10_000, { retryCount: Infinity }, async (signal) => {
    const updates = await redis.smembers(`post:sync:updates:${postId}`);
    if (updates.length === 0) {
      return;
    }

    await db.transaction(async (tx) => {
      const post = await tx
        .select({
          id: Posts.id,
          update: PostContents.update,
          text: PostContents.text,
          siteId: Entities.siteId,
        })
        .from(Posts)
        .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
        .innerJoin(Entities, eq(Posts.entityId, Entities.id))
        .where(eq(Posts.id, postId))
        .then(firstOrThrow);

      const pendingUpdates = R.groupBy(
        updates.map((update) => JSON.parse(update) as { userId: string; data: string }),
        ({ userId }) => userId,
      );

      const doc = new Y.Doc({ gc: false });
      Y.applyUpdateV2(doc, post.update);

      let prevCharacterCount = getCharacterCount(post.text);
      let order = 0;

      signal.throwIfAborted();

      for (const [userId, data] of Object.entries(pendingUpdates)) {
        const prevSnapshot = Y.snapshot(doc);
        const update = Y.mergeUpdatesV2(data.map(({ data }) => base64.parse(data)));

        Y.applyUpdateV2(doc, update);
        const snapshot = Y.snapshot(doc);

        if (!Y.equalSnapshots(prevSnapshot, snapshot)) {
          snapshotUpdated = true;

          const postSnapshot = await tx
            .insert(PostSnapshots)
            .values({
              postId,
              snapshot: Y.encodeSnapshotV2(snapshot),
              order: order++,
            })
            .returning({ id: PostSnapshots.id })
            .then(firstOrThrow);

          await tx.insert(PostSnapshotContributors).values({
            snapshotId: postSnapshot.id,
            userId,
          });

          const fragment = doc.getXmlFragment('body');
          const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
          const body = node.toJSON();
          const text = makeText(body);

          const characterCount = getCharacterCount(text);
          const characterCountDelta = characterCount - prevCharacterCount;

          if (characterCountDelta !== 0) {
            await tx
              .insert(PostCharacterCountChanges)
              .values({
                postId,
                userId,
                bucket: dayjs().startOf('hour'),
                additions: Math.max(characterCountDelta, 0),
                deletions: Math.max(-characterCountDelta, 0),
              })
              .onConflictDoUpdate({
                target: [PostCharacterCountChanges.userId, PostCharacterCountChanges.postId, PostCharacterCountChanges.bucket],
                set: {
                  additions: characterCountDelta > 0 ? sql`${PostCharacterCountChanges.additions} + ${characterCountDelta}` : undefined,
                  deletions: characterCountDelta < 0 ? sql`${PostCharacterCountChanges.deletions} + ${-characterCountDelta}` : undefined,
                },
              });
          }

          prevCharacterCount = characterCount;
        }
      }

      signal.throwIfAborted();

      await tx
        .update(PostContents)
        .set({
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        })
        .where(eq(PostContents.postId, postId));

      if (snapshotUpdated) {
        signal.throwIfAborted();

        const map = doc.getMap('attrs');

        const title = (map.get('title') as string) || null;
        const subtitle = (map.get('subtitle') as string) || null;
        const maxWidth = (map.get('maxWidth') as number) ?? 800;
        const coverImageId = JSON.parse((map.get('coverImage') as string) || '{}')?.id ?? null;
        const note = (map.get('note') as string) || '';
        const storedMarks = (map.get('storedMarks') as unknown[]) ?? [];
        const anchors = (map.get('anchors') as Record<string, string | null>) ?? {};

        const fragment = doc.getXmlFragment('body');
        const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
        const body = node.toJSON();
        const text = makeText(body);

        const characterCount = getCharacterCount(text);
        const blobSize = getBlobSize(node);

        const updatedAt = dayjs();

        await tx
          .update(Posts)
          .set({
            title,
            subtitle,
            maxWidth,
            coverImageId,
            updatedAt,
          })
          .where(eq(Posts.id, postId));

        await tx
          .update(PostContents)
          .set({
            body,
            text,
            characterCount,
            blobSize,
            storedMarks,
            note,
            updatedAt,
          })
          .where(eq(PostContents.postId, postId));

        await tx.delete(PostAnchors).where(and(eq(PostAnchors.postId, postId), notInArray(PostAnchors.nodeId, Object.keys(anchors))));

        for (const [nodeId, name] of Object.entries(anchors)) {
          await tx
            .insert(PostAnchors)
            .values({ postId, nodeId, name })
            .onConflictDoUpdate({
              target: [PostAnchors.postId, PostAnchors.nodeId],
              set: { name },
            });
        }
      }

      signal.throwIfAborted();
    });

    await redis.srem(`post:sync:updates:${postId}`, ...updates);
  });

  const updatesLeft = await redis.scard(`post:sync:updates:${postId}`);
  if (updatesLeft > 0) {
    await queue.removeDeduplicationKey(`post:sync:collect:${postId}`);
    await enqueueJob('post:sync:collect', postId, {
      deduplication: {
        id: `post:sync:collect:${postId}`,
      },
    });
  }

  if (snapshotUpdated) {
    const { siteId, entityId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id })
      .from(Posts)
      .innerJoin(Entities, eq(Posts.entityId, Entities.id))
      .where(eq(Posts.id, postId))
      .then(firstOrThrow);

    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    pubsub.publish('site:usage:update', siteId, null);

    await enqueueJob('post:index', postId, {
      deduplication: {
        id: `post:index:${postId}`,
      },
    });
  }
});

export const PostIndexJob = defineJob('post:index', async (postId: string) => {
  const post = await db
    .select({
      id: Posts.id,
      state: Entities.state,
      siteId: Entities.siteId,
      title: Posts.title,
      subtitle: Posts.subtitle,
      text: PostContents.text,
    })
    .from(Posts)
    .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
    .innerJoin(Entities, eq(Posts.entityId, Entities.id))
    .where(eq(Posts.id, postId))
    .then(firstOrThrow);

  if (post.state === EntityState.ACTIVE) {
    await meilisearch.index('posts').addDocuments([
      {
        id: post.id,
        siteId: post.siteId,
        title: post.title,
        subtitle: post.subtitle,
        text: post.text,
      },
    ]);
  } else {
    await meilisearch.index('posts').deleteDocument(post.id);
  }
});

type Snapshot = { id: string; createdAt: dayjs.Dayjs; userIds: Set<string> };

export const PostCompactJob = defineJob('post:compact', async (postId: string) => {
  await redlock.using([`{lock}:post:${postId}`], 30 * 60 * 1000, { retryCount: 5 }, async (signal) => {
    await db.transaction(async (tx) => {
      const snapshots = await tx
        .select({ id: PostSnapshots.id, createdAt: PostSnapshots.createdAt })
        .from(PostSnapshots)
        .where(eq(PostSnapshots.postId, postId))
        .orderBy(asc(PostSnapshots.createdAt), asc(PostSnapshots.order));

      signal.throwIfAborted();

      if (snapshots.length === 0) {
        await tx.update(PostContents).set({ compactedAt: dayjs() }).where(eq(PostContents.postId, postId));
        return;
      }

      const contributors = await tx
        .select({ snapshotId: PostSnapshotContributors.snapshotId, userId: PostSnapshotContributors.userId })
        .from(PostSnapshotContributors)
        .innerJoin(PostSnapshots, eq(PostSnapshotContributors.snapshotId, PostSnapshots.id))
        .where(eq(PostSnapshots.postId, postId));

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

      signal.throwIfAborted();

      if (retainedSnapshots.length === snapshots.length) {
        await tx.update(PostContents).set({ compactedAt: dayjs() }).where(eq(PostContents.postId, postId));
        return;
      }

      const content = await tx
        .select({ update: PostContents.update })
        .from(PostContents)
        .where(eq(PostContents.postId, postId))
        .then(firstOrThrow);

      const oldDoc = new Y.Doc({ gc: false });
      Y.applyUpdateV2(oldDoc, content.update);

      signal.throwIfAborted();

      await tx.delete(PostSnapshots).where(
        and(
          eq(PostSnapshots.postId, postId),
          notInArray(
            PostSnapshots.id,
            retainedSnapshots.map(({ id }) => id),
          ),
        ),
      );

      const newDoc = new Y.Doc({ gc: false });
      let index = 0;

      for (const snapshot of retainedSnapshots) {
        signal.throwIfAborted();

        const { snapshot: snapshotData } = await tx
          .delete(PostSnapshots)
          .where(eq(PostSnapshots.id, snapshot.id))
          .returning({ snapshot: PostSnapshots.snapshot })
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

        const postSnapshot = await tx
          .insert(PostSnapshots)
          .values({
            postId,
            snapshot: Y.encodeSnapshotV2(Y.snapshot(newDoc)),
            createdAt: snapshot.createdAt,
            order: 0,
          })
          .returning({ id: PostSnapshots.id })
          .then(firstOrThrow);

        if (snapshot.userIds.size > 0) {
          await tx.insert(PostSnapshotContributors).values(
            [...snapshot.userIds].map((userId) => ({
              snapshotId: postSnapshot.id,
              userId,
            })),
          );
        }

        index++;
      }

      signal.throwIfAborted();

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
        await tx.insert(PostSnapshots).values({
          postId,
          snapshot: Y.encodeSnapshotV2(afterSnapshot),
          order: 0,
        });
      }

      signal.throwIfAborted();

      await tx
        .update(PostContents)
        .set({
          update: Y.encodeStateAsUpdateV2(newDoc),
          vector: Y.encodeStateVector(newDoc),
          compactedAt: dayjs(),
        })
        .where(eq(PostContents.postId, postId));
    });
  });
});

export const PostCompactScanCron = defineCron('post:compact:scan', '0 * * * *', async () => {
  const threshold = dayjs().subtract(24, 'hours');

  const posts = await db
    .select({ postId: PostContents.postId })
    .from(PostContents)
    .where(and(lt(PostContents.updatedAt, threshold), lt(PostContents.compactedAt, PostContents.updatedAt)));

  await Promise.all(
    posts.map(({ postId }) =>
      enqueueJob('post:compact', postId, {
        delay: Math.random() * 10 * 60 * 1000,
        priority: 1,
      }),
    ),
  );
});
