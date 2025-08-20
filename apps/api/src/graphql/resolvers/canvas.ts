import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, inArray, isNull } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { base64 } from 'rfc4648';
import * as Y from 'yjs';
import { redis } from '@/cache';
import {
  CanvasContents,
  Canvases,
  CanvasSnapshotContributors,
  CanvasSnapshots,
  db,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  TableCode,
  validateDbId,
} from '@/db';
import { CanvasSyncType, EntityAvailability, EntityState, EntityType } from '@/enums';
import { NotFoundError } from '@/errors';
import { enqueueJob } from '@/mq';
import { pubsub } from '@/pubsub';
import { generateEntityOrder, generatePermalink, generateSlug, makeCanvasYDoc } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Canvas, CanvasSnapshot, CanvasView, Entity, EntityView, ICanvas, isTypeOf } from '../objects';
import type { CanvasShape } from '@/db/schemas/json';

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
  createCanvas: t.withAuth({ session: true }).fieldWithInput({
    type: Canvas,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      const title = null;
      const shapes: CanvasShape[] = [];

      const doc = makeCanvasYDoc({ title, shapes });
      const snapshot = Y.snapshot(doc);

      let depth = 0;
      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ id: Entities.id, depth: Entities.depth })
          .from(Entities)
          .where(
            and(
              eq(Entities.siteId, input.siteId),
              eq(Entities.id, input.parentEntityId),
              eq(Entities.type, EntityType.FOLDER),
              eq(Entities.state, EntityState.ACTIVE),
            ),
          )
          .then(firstOrThrow);

        depth = parentEntity.depth + 1;
      }

      const last = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, input.siteId),
            input.parentEntityId ? eq(Entities.parentId, input.parentEntityId) : isNull(Entities.parentId),
          ),
        )
        .orderBy(desc(Entities.order))
        .limit(1)
        .then(first);

      const canvas = await db.transaction(async (tx) => {
        const entity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: input.siteId,
            parentId: input.parentEntityId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.CANVAS,
            order: generateEntityOrder({ lower: last?.order, upper: null }),
            depth,
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const canvas = await tx
          .insert(Canvases)
          .values({
            entityId: entity.id,
            title,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(CanvasContents).values({
          canvasId: canvas.id,
          shapes,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });

        const canvasSnapshot = await tx
          .insert(CanvasSnapshots)
          .values({
            canvasId: canvas.id,
            snapshot: Y.encodeSnapshotV2(snapshot),
          })
          .returning({ id: CanvasSnapshots.id })
          .then(firstOrThrow);

        await tx.insert(CanvasSnapshotContributors).values({
          snapshotId: canvasSnapshot.id,
          userId: ctx.session.userId,
        });

        return canvas;
      });

      pubsub.publish('site:update', input.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', input.siteId, null);

      await enqueueJob('canvas:index', canvas.id);

      return canvas;
    },
  }),

  duplicateCanvas: t.withAuth({ session: true }).fieldWithInput({
    type: Canvas,
    input: {
      canvasId: t.input.id({ validate: validateDbId(TableCode.CANVASES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({
          siteId: Entities.siteId,
          parentEntityId: Entities.parentId,
          order: Entities.order,
          depth: Entities.depth,
        })
        .from(Entities)
        .innerJoin(Canvases, eq(Entities.id, Canvases.entityId))
        .where(eq(Canvases.id, input.canvasId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      const nextEntity = await db
        .select({ order: Entities.order })
        .from(Entities)
        .where(
          and(
            eq(Entities.siteId, entity.siteId),
            entity.parentEntityId ? eq(Entities.parentId, entity.parentEntityId) : isNull(Entities.parentId),
            gt(Entities.order, entity.order),
          ),
        )
        .orderBy(asc(Entities.order))
        .limit(1)
        .then(first);

      const canvas = await db
        .select({
          title: Canvases.title,
          content: {
            shapes: CanvasContents.shapes,
          },
        })
        .from(Canvases)
        .innerJoin(CanvasContents, eq(Canvases.id, CanvasContents.canvasId))
        .where(eq(Canvases.id, input.canvasId))
        .then(firstOrThrow);

      const title = `(사본) ${canvas.title || '(제목 없음)'}`;

      const doc = makeCanvasYDoc({
        title,
        shapes: canvas.content.shapes,
      });

      const snapshot = Y.snapshot(doc);

      const newCanvas = await db.transaction(async (tx) => {
        const newEntity = await tx
          .insert(Entities)
          .values({
            userId: ctx.session.userId,
            siteId: entity.siteId,
            parentId: entity.parentEntityId,
            slug: generateSlug(),
            permalink: generatePermalink(),
            type: EntityType.CANVAS,
            order: generateEntityOrder({ lower: entity.order, upper: nextEntity?.order }),
            depth: entity.depth,
          })
          .returning({ id: Entities.id })
          .then(firstOrThrow);

        const newCanvas = await tx
          .insert(Canvases)
          .values({
            entityId: newEntity.id,
            title,
          })
          .returning()
          .then(firstOrThrow);

        await tx.insert(CanvasContents).values({
          canvasId: newCanvas.id,
          shapes: canvas.content.shapes,
          update: Y.encodeStateAsUpdateV2(doc),
          vector: Y.encodeStateVector(doc),
        });

        const canvasSnapshot = await tx
          .insert(CanvasSnapshots)
          .values({
            canvasId: newCanvas.id,
            snapshot: Y.encodeSnapshotV2(snapshot),
          })
          .returning({ id: CanvasSnapshots.id })
          .then(firstOrThrow);

        await tx.insert(CanvasSnapshotContributors).values({
          snapshotId: canvasSnapshot.id,
          userId: ctx.session.userId,
        });

        return newCanvas;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });
      pubsub.publish('site:usage:update', entity.siteId, null);

      await enqueueJob('canvas:index', newCanvas.id);

      return newCanvas;
    },
  }),

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

      await db
        .update(Entities)
        .set({
          state: EntityState.DELETED,
          deletedAt: dayjs(),
        })
        .where(eq(Entities.id, entity.id));

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

        await redis.sadd(
          `canvas:sync:updates:${input.canvasId}`,
          JSON.stringify({
            userId: ctx.session.userId,
            data: input.data,
          }),
        );

        await enqueueJob('canvas:sync:collect', input.canvasId, {
          deduplication: {
            id: `canvas:sync:collect:${input.canvasId}`,
          },
        });
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

/**
 * * Utils
 */

const getCanvasDocument = async (canvasId: string) => {
  const { update, vector } = await db
    .select({ update: CanvasContents.update, vector: CanvasContents.vector })
    .from(CanvasContents)
    .where(eq(CanvasContents.canvasId, canvasId))
    .then(firstOrThrow);

  const updates = await redis.smembers(`canvas:sync:updates:${canvasId}`);
  if (updates.length === 0) {
    return {
      update,
      vector,
    };
  }

  const pendingUpdates = updates.map((update) => {
    const { data } = JSON.parse(update);
    return base64.parse(data);
  });

  const updatedUpdate = Y.mergeUpdatesV2([update, ...pendingUpdates]);
  const updatedVector = Y.encodeStateVectorFromUpdateV2(updatedUpdate);

  return {
    update: updatedUpdate,
    vector: updatedVector,
  };
};
