import { and, asc, eq, getTableColumns, inArray, isNotNull, ne, or, sql } from 'drizzle-orm';
import escape from 'escape-string-regexp';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, FolderOptions, Folders, PostOptions, Posts, Sites, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType, FolderVisibility, PostVisibility, SiteState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import { pubsub } from '@/pubsub';
import { generateEntityOrder } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, EntityNode, EntityView, EntityViewNode, IEntity, isTypeOf, Site, SiteView } from '../objects';

/**
 * * Types
 */

IEntity.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    permalink: t.exposeString('permalink'),
    order: t.exposeString('order'),
    depth: t.exposeInt('depth'),

    url: t.string({ resolve: (self) => `${env.USERSITE_URL.replace('*.', '')}/${self.permalink}` }),
  }),
});

Entity.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    view: t.expose('id', { type: EntityView }),

    site: t.expose('siteId', { type: Site }),
    parent: t.expose('parentId', { type: Entity, nullable: true }),

    node: t.field({
      type: EntityNode,
      resolve: async (self, _, ctx) => {
        const loader = match(self.type)
          .with(EntityType.FOLDER, () =>
            ctx.loader({
              name: 'Entity.node (Folder)',
              load: (ids) => db.select().from(Folders).where(inArray(Folders.entityId, ids)),
              key: ({ entityId }) => entityId,
            }),
          )
          .with(EntityType.POST, () =>
            ctx.loader({
              name: 'Entity.node (Post)',
              load: (ids) => db.select().from(Posts).where(inArray(Posts.entityId, ids)),
              key: ({ entityId }) => entityId,
            }),
          )
          .exhaustive();

        return await loader.load(self.id);
      },
    }),

    children: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Entity.children',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Entities)
              .where(and(inArray(Entities.parentId, ids), eq(Entities.state, EntityState.ACTIVE)))
              .orderBy(asc(Entities.order));
          },
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          key: ({ parentId }) => parentId!,
        });

        return await loader.load(self.id);
      },
    }),

    ancestors: t.field({
      type: [Entity],
      resolve: async (self) => {
        const rows = await db.execute<{ id: string }>(sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.parentId}, 0 AS depth
            FROM ${Entities}
            WHERE ${eq(Entities.id, self.id)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.parentId}, sq.depth + 1
            FROM ${Entities}
            JOIN sq ON ${Entities.id} = sq.parent_id
            WHERE sq.parent_id IS NOT NULL
          )
          SELECT id
          FROM sq
          WHERE ${ne(sql`id`, self.id)}
          ORDER BY depth DESC
        `);

        return rows.map(({ id }) => id);
      },
    }),
  }),
});

EntityView.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    site: t.expose('siteId', { type: SiteView }),

    node: t.field({
      type: EntityViewNode,
      resolve: async (self, _, ctx) => {
        const loader = match(self.type)
          .with(EntityType.FOLDER, () =>
            ctx.loader({
              name: 'EntityView.node (Folder)',
              load: (ids) => db.select().from(Folders).where(inArray(Folders.entityId, ids)),
              key: ({ entityId }) => entityId,
            }),
          )
          .with(EntityType.POST, () =>
            ctx.loader({
              name: 'EntityView.node (Post)',
              load: (ids) => db.select().from(Posts).where(inArray(Posts.entityId, ids)),
              key: ({ entityId }) => entityId,
            }),
          )
          .exhaustive();

        return await loader.load(self.id);
      },
    }),

    children: t.field({
      type: [EntityView],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'EntityView.children',
          many: true,
          load: async (ids) => {
            return await db
              .select(getTableColumns(Entities))
              .from(Entities)
              .leftJoin(Posts, eq(Entities.id, Posts.entityId))
              .leftJoin(PostOptions, eq(Posts.id, PostOptions.postId))
              .leftJoin(Folders, eq(Entities.id, Folders.entityId))
              .leftJoin(FolderOptions, eq(Folders.id, FolderOptions.folderId))
              .where(
                and(
                  inArray(Entities.parentId, ids),
                  eq(Entities.state, EntityState.ACTIVE),
                  or(
                    and(
                      eq(Entities.type, EntityType.FOLDER),
                      isNotNull(Folders.id),
                      eq(FolderOptions.visibility, FolderVisibility.UNLISTED),
                    ),
                    and(eq(Entities.type, EntityType.POST), isNotNull(Posts.id), eq(PostOptions.visibility, PostVisibility.UNLISTED)),
                  ),
                ),
              )
              .orderBy(asc(Entities.order));
          },
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          key: ({ parentId }) => parentId!,
        });

        return await loader.load(self.id);
      },
    }),

    ancestors: t.field({
      type: [EntityView],
      resolve: async (self) => {
        const rows = await db.execute<{ id: string }>(sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.parentId}, 0 AS depth
            FROM ${Entities}
            WHERE ${eq(Entities.id, self.id)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.parentId}, sq.depth + 1
            FROM ${Entities}
            LEFT JOIN ${Posts} ON ${eq(Entities.id, Posts.entityId)}
            LEFT JOIN ${PostOptions} ON ${eq(Posts.id, PostOptions.postId)}
            LEFT JOIN ${Folders} ON ${eq(Entities.id, Folders.entityId)}
            LEFT JOIN ${FolderOptions} ON ${eq(Folders.id, FolderOptions.folderId)}
            JOIN sq ON ${Entities.id} = sq.parent_id
            WHERE sq.parent_id IS NOT NULL AND
            ${or(
              and(eq(Entities.type, EntityType.FOLDER), isNotNull(Folders.id), eq(FolderOptions.visibility, FolderVisibility.UNLISTED)),
              and(eq(Entities.type, EntityType.POST), isNotNull(Posts.id), eq(PostOptions.visibility, PostVisibility.UNLISTED)),
            )}
          ) 
          SELECT id
          FROM sq
          WHERE ${ne(sql`id`, self.id)}
          ORDER BY depth DESC
        `);

        return rows.map(({ id }) => id);
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
    args: { entityId: t.arg.id({ validate: validateDbId(TableCode.ENTITIES) }) },
    resolve: async (_, args, ctx) => {
      const entity = await db
        .select()
        .from(Entities)
        .where(and(eq(Entities.id, args.entityId), eq(Entities.userId, ctx.session.userId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      return entity;
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
  moveEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      parentEntityId: t.input.id({ required: false, validate: validateDbId(TableCode.ENTITIES) }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ id: Entities.id, siteId: Entities.siteId, depth: Entities.depth })
        .from(Entities)
        .where(eq(Entities.id, input.entityId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      let depth = 0;
      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ id: Entities.id, depth: Entities.depth })
          .from(Entities)
          .where(and(eq(Entities.id, input.parentEntityId), eq(Entities.siteId, entity.siteId)))
          .then(firstOrThrow);

        const hasCycle = await db
          .execute<{ exists: boolean }>(
            sql`
              WITH RECURSIVE sq AS (
                SELECT ${Entities.id}, ${Entities.parentId}
                FROM ${Entities}
                WHERE ${eq(Entities.id, parentEntity.id)}
                UNION ALL
                SELECT ${Entities.id}, ${Entities.parentId}
                FROM ${Entities}
                JOIN sq ON ${Entities.id} = sq.parent_id
              )
              SELECT EXISTS (
                SELECT 1 FROM sq WHERE ${eq(sql`id`, entity.id)}
              ) as exists
            `,
          )
          .then(firstOrThrow);

        if (hasCycle.exists) {
          throw new TypieError({ code: 'circular_reference' });
        }

        depth = parentEntity.depth + 1;
      }

      const depthDelta = depth - entity.depth;

      const updatedEntity = await db.transaction(async (tx) => {
        const updatedEntity = await tx
          .update(Entities)
          .set({
            parentId: input.parentEntityId,
            order: generateEntityOrder({
              lower: input.lowerOrder,
              upper: input.upperOrder,
            }),
            depth,
          })
          .where(eq(Entities.id, entity.id))
          .returning()
          .then(firstOrThrow);

        await tx.execute(sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.depth}
            FROM ${Entities}
            WHERE ${eq(Entities.parentId, entity.id)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.depth}
            FROM ${Entities}
            JOIN sq ON ${Entities.parentId} = sq.id
          )
          UPDATE ${Entities}
          SET depth = depth + ${depthDelta}
          WHERE id IN (SELECT id FROM sq)
        `);

        return updatedEntity;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });

      return updatedEntity;
    },
  }),
}));
