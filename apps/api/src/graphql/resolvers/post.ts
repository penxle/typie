import dayjs from 'dayjs';
import { eq } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import { base64 } from 'rfc4648';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, firstOrThrow, PostContentSnapshots, PostContentStates, Posts } from '@/db';
import { PostContentSyncKind } from '@/enums';
import { enqueueJob } from '@/mq';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { makeYDoc } from '@/utils';
import { builder } from '../builder';
import { Post } from '../objects';

/**
 * * Types
 */

Post.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createPost: t.field({
    type: Post,
    resolve: async () => {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const node = schema.topNodeType.createAndFill()!;
      const content = node.toJSON();

      const doc = makeYDoc({
        title: null,
        subtitle: null,
        content,
      });

      const snapshot = Y.snapshot(doc);

      return await db.transaction(async (tx) => {
        const post = await tx.insert(Posts).values({}).returning().then(firstOrThrow);

        await tx.insert(PostContentStates).values({
          postId: post.id,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });

        await tx.insert(PostContentSnapshots).values({
          postId: post.id,
          snapshot: Y.encodeSnapshotV2(snapshot),
        });

        return post;
      });
    },
  }),

  syncPostContent: t.fieldWithInput({
    type: [
      builder.simpleObject('SyncPostContentPayload', {
        fields: (t) => ({
          kind: t.field({ type: PostContentSyncKind }),
          data: t.field({ type: 'Binary' }),
        }),
      }),
    ],
    input: {
      postId: t.input.id(),
      kind: t.input.field({ type: PostContentSyncKind }),
      data: t.input.field({ type: 'Binary' }),
    },
    resolve: async (_, { input }) => {
      if (input.kind === PostContentSyncKind.UPDATE) {
        const data = base64.stringify(input.data);

        pubsub.publish('post:content:sync', input.postId, {
          kind: PostContentSyncKind.UPDATE,
          data,
        });

        await redis.sadd(`post:content:updates:${input.postId}`, data);

        await enqueueJob('post:content:state-update', input.postId);

        return [];
      } else if (input.kind === PostContentSyncKind.VECTOR) {
        const state = await getLatestPostContentState(input.postId);

        const clientStateVector = input.data;
        const clientMissingUpdate = Y.diffUpdateV2(state.update, clientStateVector);
        const serverStateVector = state.vector;

        return [
          { kind: PostContentSyncKind.UPDATE, data: clientMissingUpdate },
          { kind: PostContentSyncKind.VECTOR, data: serverStateVector },
        ] as const;
      } else if (input.kind === PostContentSyncKind.AWARENESS) {
        pubsub.publish('post:content:sync', input.postId, {
          kind: PostContentSyncKind.AWARENESS,
          data: base64.stringify(input.data),
        });

        return [];
      }

      throw new Error('Invalid kind');
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  postContentSyncStream: t.field({
    type: builder.simpleObject('PostContentSyncStreamPayload', {
      fields: (t) => ({
        postId: t.id(),
        kind: t.field({ type: PostContentSyncKind }),
        data: t.field({ type: 'Binary' }),
      }),
    }),
    args: { postId: t.arg.id() },
    subscribe: async (_, args, ctx) => {
      const repeater = Repeater.merge([
        pubsub.subscribe('post:content:sync', args.postId),
        new Repeater<{ postId: string; kind: PostContentSyncKind; data: string }>(async (push, stop) => {
          const ping = () => {
            push({
              postId: args.postId,
              kind: 'HEARTBEAT',
              data: base64.stringify(encoder.encode(dayjs().toISOString())),
            });
          };

          ping();
          const interval = setInterval(() => ping(), 1000);

          await stop;
          clearInterval(interval);
        }),
      ]);

      ctx.req.signal.addEventListener('abort', () => {
        repeater.return();
      });

      return repeater;
    },
    resolve: (payload, args) => ({
      postId: args.postId,
      kind: payload.kind,
      data: base64.parse(payload.data),
    }),
  }),
}));

/**
 * * Utils
 */

const encoder = new TextEncoder();

export const getLatestPostContentState = async (postId: string) => {
  const state = await db
    .select({ update: PostContentStates.update, vector: PostContentStates.vector })
    .from(PostContentStates)
    .where(eq(PostContentStates.postId, postId))
    .then(firstOrThrow);

  const pendingUpdates = await redis.smembers(`post:content:updates:${postId}`);

  if (pendingUpdates.length === 0) {
    return {
      update: state.update,
      vector: state.vector,
    };
  }

  const updatedUpdate = Y.mergeUpdatesV2([state.update, ...pendingUpdates.map((update) => base64.parse(update))]);
  const updatedVector = Y.encodeStateVectorFromUpdateV2(updatedUpdate);

  return {
    update: updatedUpdate,
    vector: updatedVector,
  };
};
