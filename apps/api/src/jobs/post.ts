import dayjs from 'dayjs';
import { and, desc, eq, gt } from 'drizzle-orm';
import * as Y from 'yjs';
import { db, firstOrThrow, PostContentStates, PostContentUpdates } from '@/db';
import { defineJob } from './types';

export const PostContentStateUpdateJob = defineJob('post:content:state-update', async (postId: string) => {
  await db.transaction(async (tx) => {
    const state = await tx
      .select({
        update: PostContentStates.update,
        vector: PostContentStates.vector,
        seq: PostContentStates.seq,
      })
      .from(PostContentStates)
      .where(eq(PostContentStates.postId, postId))
      .for('update')
      .then(firstOrThrow);

    const updates = await tx
      .select({ update: PostContentUpdates.update, seq: PostContentUpdates.seq })
      .from(PostContentUpdates)
      .where(and(eq(PostContentUpdates.postId, postId), gt(PostContentUpdates.seq, state.seq)))
      .orderBy(desc(PostContentUpdates.seq));

    if (updates.length === 0) {
      return false;
    }

    const update = Y.mergeUpdatesV2(updates.map(({ update }) => update));
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
        seq: updates[0].seq,
      })
      .where(and(eq(PostContentStates.postId, postId)));

    if (Y.equalSnapshots(prevSnapshot, snapshot)) {
      return;
    }

    await tx
      .update(PostContentStates)
      .set({
        updatedAt: dayjs(),
      })
      .where(and(eq(PostContentStates.postId, postId)));
  });
});
