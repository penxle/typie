import dayjs from 'dayjs';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { base64 } from 'rfc4648';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { CanvasSyncType } from '@/enums';
import { pubsub } from '@/pubsub';
import { builder } from '../builder';

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  syncCanvas: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      clientId: t.input.string(),
      canvasId: t.input.string(),
      type: t.input.field({ type: CanvasSyncType }),
      data: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      if (input.type === CanvasSyncType.UPDATE) {
        pubsub.publish('canvas:sync', input.canvasId, {
          target: `!${input.clientId}`,
          type: CanvasSyncType.UPDATE,
          data: input.data,
        });

        await redis.sadd(
          `canvas:sync:updates:${input.canvasId}`,
          JSON.stringify({
            userId: ctx.session.userId,
            data: input.data,
          }),
        );
      } else if (input.type === CanvasSyncType.VECTOR) {
        const state = await getCanvasDocument(input.canvasId);
        const update = Y.diffUpdateV2(state.update, base64.parse(input.data));

        pubsub.publish('canvas:sync', input.canvasId, {
          target: input.clientId,
          type: CanvasSyncType.UPDATE,
          data: base64.stringify(update),
        });

        pubsub.publish('canvas:sync', input.canvasId, {
          target: input.clientId,
          type: CanvasSyncType.VECTOR,
          data: base64.stringify(state.vector),
        });
      } else if (input.type === CanvasSyncType.AWARENESS) {
        pubsub.publish('canvas:sync', input.canvasId, {
          target: `!${input.clientId}`,
          type: CanvasSyncType.AWARENESS,
          data: input.data,
        });
      }

      return true;
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  canvasSyncStream: t.withAuth({ session: true }).field({
    type: t.builder.simpleObject('CanvasSyncStreamPayload', {
      fields: (t) => ({
        canvasId: t.id(),
        type: t.field({ type: CanvasSyncType }),
        data: t.string(),
      }),
    }),
    args: {
      clientId: t.arg.string(),
      canvasId: t.arg.id(),
    },
    subscribe: async (_, args) => {
      pubsub.publish('canvas:sync', args.canvasId, {
        target: `!${args.clientId}`,
        type: CanvasSyncType.PRESENCE,
        data: '',
      });

      const repeater = Repeater.merge([
        pubsub.subscribe('canvas:sync', args.canvasId),
        new Repeater<{ target: string; type: CanvasSyncType; data: string }>(async (push, stop) => {
          const heartbeat = () => {
            push({
              target: args.clientId,
              type: CanvasSyncType.HEARTBEAT,
              data: dayjs().toISOString(),
            });
          };

          heartbeat();
          const interval = setInterval(heartbeat, 1000);

          await stop;

          clearInterval(interval);
        }),
      ]);

      return pipe(
        repeater,
        filter(({ target }) => {
          if (target === '*') {
            return true;
          } else if (target.startsWith('!')) {
            return target.slice(1) !== args.clientId;
          } else {
            return target === args.clientId;
          }
        }),
      );
    },
    resolve: async (payload, args) => {
      return {
        canvasId: args.canvasId,
        type: payload.type,
        data: payload.data,
      };
    },
  }),
}));

/**
 * * Utils
 */

const getCanvasDocument = async (canvasId: string) => {
  const updates = await redis.smembers(`canvas:sync:updates:${canvasId}`);

  const pendingUpdates = updates.map((update) => {
    const { data } = JSON.parse(update);
    return base64.parse(data);
  });

  const updatedUpdate = Y.mergeUpdatesV2([...pendingUpdates]);
  const updatedVector = Y.encodeStateVectorFromUpdateV2(updatedUpdate);

  return {
    update: updatedUpdate,
    vector: updatedVector,
  };
};
