import dayjs from 'dayjs';
import { and, eq, sql } from 'drizzle-orm';
import * as R from 'remeda';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, Entities, firstOrThrow, PostContentDailyStats, PostContents, PostContentSnapshots, Posts } from '@/db';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { decode, makeText } from '@/utils';
import { defineJob } from '../types';

export const PostContentUpdateJob = defineJob('post:content:update', async (postId: string) => {
  const buffers = await redis.smembersBuffer(`post:content:updates:${postId}`);
  if (buffers.length === 0) {
    return;
  }

  let snapshotUpdated = false;

  await db.transaction(async (tx) => {
    const state = await tx
      .select({
        id: PostContents.id,
        update: PostContents.update,
        vector: PostContents.vector,
        text: PostContents.text,
      })
      .from(PostContents)
      .where(eq(PostContents.postId, postId))
      .for('update')
      .then(firstOrThrow);

    const updateDataByUser = R.groupBy(
      buffers
        .map((raw) => {
          const data = new Uint8Array(raw);
          const sepIdx = data.indexOf(0);

          const userId = decode(data.slice(0, sepIdx));
          const update = data.slice(sepIdx + 1);
          return { userId, update, raw };
        })
        .filter((result) => result !== null),
      (result) => result.userId,
    );

    const doc = new Y.Doc({ gc: false });
    Y.applyUpdateV2(doc, state.update);

    let previousTextLength = state.text.length;
    let order = 0;

    for (const [userId, data] of Object.entries(updateDataByUser)) {
      const prevSnapshot = Y.snapshot(doc);
      const update = Y.mergeUpdatesV2(data.map((result) => result.update));

      Y.applyUpdateV2(doc, update);
      const snapshot = Y.snapshot(doc);

      if (!Y.equalSnapshots(prevSnapshot, snapshot)) {
        snapshotUpdated = true;

        await tx.insert(PostContentSnapshots).values({
          postId,
          userId,
          snapshot: Y.encodeSnapshotV2(snapshot),
          order,
        });

        const fragment = doc.getXmlFragment('body');
        const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
        const body = node.toJSON();
        const text = makeText(body);

        const textLengthDiff = text.length - previousTextLength;
        if (textLengthDiff > 0) {
          await tx
            .insert(PostContentDailyStats)
            .values({
              postContentId: state.id,
              userId,
              date: dayjs().kst().startOf('day'),
              characters: textLengthDiff,
            })
            .onConflictDoUpdate({
              target: [PostContentDailyStats.userId, PostContentDailyStats.postContentId, PostContentDailyStats.date],
              set: {
                characters: sql`${PostContentDailyStats.characters} + ${textLengthDiff}`,
              },
            });
        }

        previousTextLength = text.length;
      }

      order++;
    }

    await tx
      .update(PostContents)
      .set({
        update: Y.encodeStateAsUpdateV2(doc),
        vector: Y.encodeStateVector(doc),
      })
      .where(eq(PostContents.postId, postId));

    if (snapshotUpdated) {
      const map = doc.getMap('attrs');

      const title = (map.get('title') as string) || null;
      const subtitle = (map.get('subtitle') as string) || null;
      const maxWidth = (map.get('maxWidth') as number) ?? 1000;
      const coverImageId = JSON.parse((map.get('coverImage') as string) || '{}')?.id ?? null;

      const fragment = doc.getXmlFragment('body');
      const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
      const body = node.toJSON();
      const text = makeText(body);

      await tx
        .update(PostContents)
        .set({
          title,
          subtitle,
          body,
          text,
          maxWidth,
          coverImageId,
          updatedAt: dayjs(),
        })
        .where(and(eq(PostContents.postId, postId)));
    }

    return buffers;
  });

  await redis.srem(`post:content:updates:${postId}`, ...buffers);

  if (snapshotUpdated) {
    const { siteId, entityId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id })
      .from(Posts)
      .innerJoin(Entities, eq(Posts.entityId, Entities.id))
      .where(eq(Posts.id, postId))
      .then(firstOrThrow);

    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
  }
});
