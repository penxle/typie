import dayjs from 'dayjs';
import { and, asc, eq, inArray } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import * as Y from 'yjs';
import { redis } from '@/cache';
import {
  CanvasContents,
  Canvases,
  CanvasSnapshots,
  db,
  Entities,
  firstOrThrow,
  firstOrThrowWith,
  Notes,
  TableCode,
  validateDbId,
} from '@/db';
import { CanvasSyncType, EntityAvailability, EntityState, NoteState } from '@/enums';
import { NotFoundError } from '@/errors';
import { enqueueJob } from '@/mq';
import { pubsub } from '@/pubsub';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Canvas, CanvasSnapshot, CanvasView, Entity, EntityView, ICanvas, isTypeOf } from '../objects';

/**
 * * Types
 */

ICanvas.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    title: t.string({ resolve: (self) => self.title || '(제목 없음)' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

Canvas.implement({
  isTypeOf: isTypeOf(TableCode.CANVASES),
  interfaces: [ICanvas],
  fields: (t) => ({
    view: t.expose('id', { type: CanvasView }),

    update: t.field({
      type: 'Binary',
      resolve: async (self) => {
        const content = await db
          .select({ update: CanvasContents.update })
          .from(CanvasContents)
          .where(eq(CanvasContents.canvasId, self.id))
          .then(firstOrThrow);

        return content.update;
      },
    }),

    snapshots: t.field({
      type: [CanvasSnapshot],
      resolve: async (self) => {
        return await db.select().from(CanvasSnapshots).where(eq(CanvasSnapshots.canvasId, self.id)).orderBy(asc(CanvasSnapshots.createdAt));
      },
    }),

    entity: t.expose('entityId', { type: Entity }),
  }),
});

CanvasView.implement({
  isTypeOf: isTypeOf(TableCode.CANVASES),
  interfaces: [ICanvas],
  fields: (t) => ({
    shapes: t.field({
      type: 'JSON',
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'CanvasView.shapes',
          load: async (ids) => {
            return await db
              .select({ canvasId: CanvasContents.canvasId, shapes: CanvasContents.shapes })
              .from(CanvasContents)
              .where(inArray(CanvasContents.canvasId, ids));
          },
          key: ({ canvasId }) => canvasId,
        });

        const content = await loader.load(self.id);
        return content.shapes;
      },
    }),

    entity: t.expose('entityId', { type: EntityView }),
  }),
});

CanvasSnapshot.implement({
  isTypeOf: isTypeOf(TableCode.CANVAS_SNAPSHOTS),
  fields: (t) => ({
    id: t.exposeID('id'),
    snapshot: t.expose('snapshot', { type: 'Binary' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  canvas: t.withAuth({ session: true }).field({
    type: Canvas,
    args: { slug: t.arg.string() },
    resolve: async (_, args, ctx) => {
      const { canvas, entity } = await db
        .select({ canvas: Canvases, entity: { siteId: Entities.siteId, availability: Entities.availability } })
        .from(Canvases)
        .innerJoin(Entities, eq(Canvases.entityId, Entities.id))
        .where(eq(Entities.slug, args.slug))
        .then(firstOrThrowWith(new NotFoundError()));

      if (entity.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: entity.siteId,
        }).catch(() => {
          throw new NotFoundError();
        });
      }

      return canvas;
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  deleteCanvas: t.withAuth({ session: true }).fieldWithInput({
    type: Canvas,
    input: { canvasId: t.input.id({ validate: validateDbId(TableCode.CANVASES) }) },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ id: Entities.id, siteId: Entities.siteId })
        .from(Entities)
        .innerJoin(Canvases, eq(Entities.id, Canvases.entityId))
        .where(eq(Canvases.id, input.canvasId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({
            state: EntityState.DELETED,
            deletedAt: dayjs(),
          })
          .where(eq(Entities.id, entity.id));

        await tx
          .update(Notes)
          .set({ state: NoteState.DELETED_CASCADED })
          .where(and(eq(Notes.entityId, entity.id), eq(Notes.state, NoteState.ACTIVE)));
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:update', entity.siteId, { scope: 'entity', entityId: entity.id });
      pubsub.publish('site:usage:update', entity.siteId, null);

      await enqueueJob('canvas:index', input.canvasId);

      return input.canvasId;
    },
  }),

  syncCanvas: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: {
      clientId: t.input.string(),
      canvasId: t.input.id({ validate: validateDbId(TableCode.CANVASES) }),
      type: t.input.field({ type: CanvasSyncType }),
      data: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const canvas = await db
        .select({ siteId: Entities.siteId, availability: Entities.availability })
        .from(Canvases)
        .innerJoin(Entities, eq(Canvases.entityId, Entities.id))
        .where(eq(Canvases.id, input.canvasId))
        .then(firstOrThrow);

      if (canvas.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: canvas.siteId,
        });
      }

      if (input.type === CanvasSyncType.UPDATE) {
        pubsub.publish('canvas:sync', input.canvasId, {
          target: `!${input.clientId}`,
          type: CanvasSyncType.UPDATE,
          data: input.data,
        });

        await redis.lpush(
          `canvas:sync:updates:${input.canvasId}`,
          JSON.stringify({
            userId: ctx.session.userId,
            data: input.data,
          }),
        );

        await enqueueJob('canvas:sync:collect', input.canvasId);
      } else if (input.type === CanvasSyncType.VECTOR) {
        const contents = await db
          .select({ update: CanvasContents.update, vector: CanvasContents.vector })
          .from(CanvasContents)
          .where(eq(CanvasContents.canvasId, input.canvasId))
          .then(firstOrThrow);

        const update = Y.diffUpdateV2(contents.update, Uint8Array.fromBase64(input.data));

        pubsub.publish('canvas:sync', input.canvasId, {
          target: input.clientId,
          type: CanvasSyncType.UPDATE,
          data: update.toBase64(),
        });

        pubsub.publish('canvas:sync', input.canvasId, {
          target: input.clientId,
          type: CanvasSyncType.VECTOR,
          data: contents.vector.toBase64(),
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
      canvasId: t.arg.id({ validate: validateDbId(TableCode.CANVASES) }),
    },
    subscribe: async (_, args, ctx) => {
      const canvas = await db
        .select({ siteId: Entities.siteId, availability: Entities.availability })
        .from(Canvases)
        .innerJoin(Entities, eq(Canvases.entityId, Entities.id))
        .where(eq(Canvases.id, args.canvasId))
        .then(firstOrThrow);

      if (canvas.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: canvas.siteId,
        });
      }

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
