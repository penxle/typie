import { and, asc, eq, ne, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import escape from 'escape-string-regexp';
import { generateJitteredKeyBetween } from 'fractional-indexing-jittered';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, Folders, Posts, Sites, TableCode } from '@/db';
import { EntityState, EntityType, SiteState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import { pubsub } from '@/pubsub';
import { decode, encode } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, EntityNode, EntityView, EntityViewNode, IEntity, isTypeOf, Site } from '../objects';

/**
 * * Types
 */

IEntity.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    permalink: t.exposeString('permalink'),
    order: t.expose('order', { type: 'Binary' }),

    site: t.field({ type: Site, resolve: (self) => self.siteId }),
  }),
});

Entity.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    node: t.field({
      type: EntityNode,
      resolve: async (self) => {
        return match(self.type)
          .with(EntityType.FOLDER, () => db.select().from(Folders).where(eq(Folders.entityId, self.id)).then(firstOrThrow))
          .with(EntityType.POST, () => db.select().from(Posts).where(eq(Posts.entityId, self.id)).then(firstOrThrow))
          .exhaustive();
      },
    }),

    children: t.field({
      type: [Entity],
      resolve: async (self) => {
        return await db
          .select()
          .from(Entities)
          .where(and(eq(Entities.parentId, self.id), eq(Entities.state, EntityState.ACTIVE)))
          .orderBy(asc(Entities.order));
      },
    }),

    ancestors: t.field({
      type: [Entity],
      resolve: async (self) => {
        const e = alias(Entities, 'e');

        const result = await db.execute<{ id: string }>(sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.parentId}, 0 AS depth
            FROM ${Entities}
            WHERE ${eq(Entities.id, self.id)}
            UNION ALL
            SELECT ${e.id}, ${e.parentId}, sq.depth + 1
            FROM ${Entities} as e
            JOIN sq ON ${e.id} = sq.parent_id
            WHERE sq.parent_id IS NOT NULL
          )
          SELECT id
          FROM sq
          WHERE ${ne(sql`id`, self.id)}
          ORDER BY depth DESC
        `);

        return result.map((row) => row.id);
      },
    }),
  }),
});

EntityView.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    node: t.field({
      type: EntityViewNode,
      resolve: async (self) => {
        return match(self.type)
          .with(EntityType.FOLDER, () => db.select().from(Folders).where(eq(Folders.entityId, self.id)).then(firstOrThrow))
          .with(EntityType.POST, () => db.select().from(Posts).where(eq(Posts.entityId, self.id)).then(firstOrThrow))
          .exhaustive();
      },
    }),

    children: t.field({
      type: [EntityView],
      resolve: async (self) => {
        return await db
          .select()
          .from(Entities)
          .where(and(eq(Entities.parentId, self.id), eq(Entities.state, EntityState.ACTIVE)))
          .orderBy(asc(Entities.order));
      },
    }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  entity: t.withAuth({ session: true }).field({
    type: Entity,
    args: { id: t.arg.id() },
    resolve: async (_, args, ctx) => {
      return await db
        .select()
        .from(Entities)
        .where(and(eq(Entities.id, args.id), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);
    },
  }),

  entityView: t.field({
    type: EntityView,
    args: { origin: t.arg.string(), slug: t.arg.string() },
    resolve: async (_, args) => {
      const pattern = new RegExp(`^${escape(env.USERSITE_URL).replace(String.raw`\*\.`, String.raw`([^.]+)\.`)}$`);
      const slug = args.origin.match(pattern)?.[1];
      if (!slug) {
        throw new TypieError({ code: 'invalid_hostname' });
      }

      const site = await db
        .select({ id: Sites.id })
        .from(Sites)
        .where(and(eq(Sites.slug, slug), eq(Sites.state, SiteState.ACTIVE)))
        .then(firstOrThrow);

      return await db
        .select()
        .from(Entities)
        .where(and(eq(Entities.siteId, site.id), eq(Entities.slug, args.slug), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);
    },
  }),

  entityViewByPermalink: t.field({
    type: EntityView,
    args: { permalink: t.arg.string() },
    resolve: async (_, args) => {
      return await db
        .select()
        .from(Entities)
        .where(and(eq(Entities.permalink, args.permalink), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);
    },
  }),
}));

builder.mutationFields((t) => ({
  updateEntityPosition: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: {
      id: t.input.id(),
      parentId: t.input.id({ required: false }),
      previousOrder: t.input.field({ type: 'Binary', required: false }),
      nextOrder: t.input.field({ type: 'Binary', required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({
          id: Entities.id,
          siteId: Entities.siteId,
        })
        .from(Entities)
        .where(and(eq(Entities.id, input.id), eq(Entities.userId, ctx.session.userId)))
        .then(firstOrThrow);

      const parentEntity = input.parentId
        ? await db
            .select({
              id: Entities.id,
              siteId: Entities.siteId,
            })
            .from(Entities)
            .where(and(eq(Entities.id, input.parentId), eq(Entities.userId, ctx.session.userId)))
            .then(firstOrThrow)
        : null;

      await assertSitePermission({ userId: ctx.session.userId, siteId: entity.siteId, ctx });

      if (parentEntity) {
        if (parentEntity.siteId !== entity.siteId) {
          throw new TypieError({ code: 'cross_site' });
        }

        const ancestors = await db.execute<{ id: string }>(sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.parentId}
            FROM ${Entities}
            WHERE ${eq(Entities.id, parentEntity.id)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.parentId}
            FROM ${Entities}
            JOIN sq ON ${Entities.id} = sq.parent_id
            WHERE sq.parent_id IS NOT NULL
          )
          SELECT id
          FROM sq
          WHERE ${eq(sql`id`, entity.id)}
        `);

        if (ancestors.length > 0) {
          throw new TypieError({ code: 'circular_reference' });
        }
      }

      const updatedEntity = await db
        .update(Entities)
        .set({
          parentId: parentEntity?.id ?? null,
          order: encode(
            generateJitteredKeyBetween(
              input.previousOrder ? decode(input.previousOrder) : null,
              input.nextOrder ? decode(input.nextOrder) : null,
            ),
          ),
        })
        .where(eq(Entities.id, entity.id))
        .returning()
        .then(firstOrThrow);

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });

      return updatedEntity;
    },
  }),
}));
