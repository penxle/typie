import { findChildren } from '@tiptap/core';
import dayjs from 'dayjs';
import { eq, sql } from 'drizzle-orm';
import * as R from 'remeda';
import { base64 } from 'rfc4648';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis, redlock } from '@/cache';
import { db, Entities, firstOrThrow, PostCharacterCountChanges, PostContents, Posts, PostSnapshots } from '@/db';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { meilisearch } from '@/search';
import { makeText } from '@/utils';
import { queue } from '../bullmq';
import { enqueueJob } from '../index';
import { defineJob } from '../types';
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

  await redlock.using([`{lock}:post:sync:collect:${postId}`], 10_000, { retryCount: Infinity }, async (signal) => {
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

          await tx.insert(PostSnapshots).values({
            postId,
            userId,
            snapshot: Y.encodeSnapshotV2(snapshot),
            order: order++,
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

  await meilisearch.index('posts').addDocuments([
    {
      id: post.id,
      siteId: post.siteId,
      title: post.title,
      subtitle: post.subtitle,
      text: post.text,
    },
  ]);
});
