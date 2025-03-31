import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { base64 } from 'rfc4648';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, firstOrThrow, PostContentSnapshots, PostContentStates } from '@/db';
import { schema } from '@/pm';
import { makeText } from '@/utils';
import { defineJob } from '../types';

export const PostContentStateUpdateJob = defineJob('post:content:state-update', async (postId: string) => {
  await db.transaction(async (tx) => {
    const state = await tx
      .select({
        update: PostContentStates.update,
        vector: PostContentStates.vector,
      })
      .from(PostContentStates)
      .where(eq(PostContentStates.postId, postId))
      .for('update')
      .then(firstOrThrow);

    const updates = await redis.smembers(`post:content:updates:${postId}`);

    if (updates.length === 0) {
      return;
    }

    const update = Y.mergeUpdatesV2(updates.map((update) => base64.parse(update)));
    const doc = new Y.Doc();

    Y.applyUpdateV2(doc, state.update);
    const prevSnapshot = Y.snapshot(doc);

    Y.applyUpdateV2(doc, update);
    const snapshot = Y.snapshot(doc);

    await tx
      .update(PostContentStates)
      .set({
        update: Y.encodeStateAsUpdateV2(doc),
        vector: Y.encodeStateVector(doc),
        updatedAt: dayjs(),
      })
      .where(and(eq(PostContentStates.postId, postId)));

    await redis.srem(`post:content:updates:${postId}`, ...updates);

    if (Y.equalSnapshots(prevSnapshot, snapshot)) {
      return;
    }

    const title = doc.getText('title').toString() || null;
    const subtitle = doc.getText('subtitle').toString() || null;
    const fragment = doc.getXmlFragment('content');

    const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
    const content = node.toJSON();
    const text = makeText(content);

    await tx
      .update(PostContentStates)
      .set({
        title,
        subtitle,
        content,
        text,
        updatedAt: dayjs(),
      })
      .where(and(eq(PostContentStates.postId, postId)));

    await tx.insert(PostContentSnapshots).values({
      postId,
      snapshot: Y.encodeSnapshotV2(snapshot),
    });
  });
});
