import dayjs from 'dayjs';
import { and, eq, gt } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import { base64 } from 'rfc4648';
import * as Y from 'yjs';
import { db, firstOrThrow, PostContentStates, PostContentUpdates } from '@/db';
import { PostContentSyncKind } from '@/enums';
import { enqueueJob } from '@/mq';
import { pubsub } from '@/pubsub';
import { makeYDoc } from '@/utils';
import { builder } from '../builder';

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  createPost: t.fieldWithInput({
    type: 'Boolean',
    input: { postId: t.input.id() },
    resolve: async (_, { input }) => {
      const doc = makeYDoc({
        title: null,
        subtitle: null,
        content: {},
      });

      await db.transaction(async (tx) => {
        await tx.insert(PostContentStates).values({
          postId: input.postId,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });
      });

      return true;
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
        pubsub.publish('post:content:sync', input.postId, {
          kind: input.kind,
          data: base64.stringify(input.data),
        });

        await db.transaction(async (tx) => {
          await tx.insert(PostContentUpdates).values({
            postId: input.postId,
            update: input.data,
          });
        });

        await enqueueJob('post:content:state-update', input.postId);

        return [];
      } else if (input.kind === PostContentSyncKind.VECTOR) {
        const state = await getLatestPostContentState(input.postId);

        const clientStateVector = input.data;
        const clientMissingUpdate = Y.diffUpdateV2(state.update, clientStateVector);
        const serverStateVector = state.vector;

        return [
          { kind: 'UPDATE', data: clientMissingUpdate },
          { kind: 'VECTOR', data: serverStateVector },
        ] as const;
      } else if (input.kind === PostContentSyncKind.AWARENESS) {
        pubsub.publish('post:content:sync', input.postId, {
          kind: input.kind,
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
    .select({ update: PostContentStates.update, vector: PostContentStates.vector, seq: PostContentStates.seq })
    .from(PostContentStates)
    .where(eq(PostContentStates.postId, postId))
    .then(firstOrThrow);

  const pendingUpdates = await db
    .select({ update: PostContentUpdates.update })
    .from(PostContentUpdates)
    .where(and(eq(PostContentUpdates.postId, postId), gt(PostContentUpdates.seq, state.seq)));

  if (pendingUpdates.length === 0) {
    return {
      update: state.update,
      vector: state.vector,
    };
  }

  const updatedUpdate = Y.mergeUpdatesV2([state.update, ...pendingUpdates.map(({ update }) => update)]);
  const updatedVector = Y.encodeStateVectorFromUpdateV2(updatedUpdate);

  return {
    update: updatedUpdate,
    vector: updatedVector,
  };
};
