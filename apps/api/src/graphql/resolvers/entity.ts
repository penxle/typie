import { and, asc, eq, ne, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import escape from 'escape-string-regexp';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, Folders, Posts, Sites, TableCode } from '@/db';
import { EntityState, EntityType, SiteState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
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
