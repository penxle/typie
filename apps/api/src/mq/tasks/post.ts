import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { base64 } from 'rfc4648';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, firstOrThrow, PostContents, PostContentSnapshots } from '@/db';
import { schema } from '@/pm';
import { makeText } from '@/utils';
import { defineJob } from '../types';

export const PostContentUpdateJob = defineJob('post:content:update', async (postId: string) => {
  await db.transaction(async (tx) => {
    const state = await tx
      .select({
        update: PostContents.update,
        vector: PostContents.vector,
      })
      .from(PostContents)
      .where(eq(PostContents.postId, postId))
      .for('update')
      .then(firstOrThrow);

    const updates = await redis.smembers(`post:content:updates:${postId}`);

    if (updates.length === 0) {
      return;
    }

    const update = Y.mergeUpdatesV2(updates.map((update) => base64.parse(update)));
    const doc = new Y.Doc({ gc: false });

    Y.applyUpdateV2(doc, state.update);
    const prevSnapshot = Y.snapshot(doc);

    Y.applyUpdateV2(doc, update);
    const snapshot = Y.snapshot(doc);

    await tx
      .update(PostContents)
      .set({
        update: Y.encodeStateAsUpdateV2(doc),
        vector: Y.encodeStateVector(doc),
        updatedAt: dayjs(),
      })
      .where(and(eq(PostContents.postId, postId)));

    await redis.srem(`post:content:updates:${postId}`, ...updates);

    if (Y.equalSnapshots(prevSnapshot, snapshot)) {
      return;
    }

    const map = doc.getMap('attrs');

    const title = (map.get('title') as string) || null;
    const subtitle = (map.get('subtitle') as string) || null;
    const maxWidth = (map.get('maxWidth') as number) ?? 1000;

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
        updatedAt: dayjs(),
      })
      .where(and(eq(PostContents.postId, postId)));

    await tx.insert(PostContentSnapshots).values({
      postId,
      snapshot: Y.encodeSnapshotV2(snapshot),
    });
  });
});
