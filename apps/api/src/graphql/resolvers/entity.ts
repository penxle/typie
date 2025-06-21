import { and, asc, eq, getTableColumns, inArray, ne, sql } from 'drizzle-orm';
import escape from 'escape-string-regexp';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, firstOrThrowWith, Folders, Posts, Sites, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType, EntityVisibility, SiteState } from '@/enums';
import { env } from '@/env';
import { NotFoundError, TypieError } from '@/errors';
import { pubsub } from '@/pubsub';
import { generateEntityOrder } from '@/utils';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, EntityNode, EntityView, EntityViewNode, IEntity, isTypeOf, Site, SiteView, User } from '../objects';

/**
 * * Types
 */

IEntity.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    state: t.expose('state', { type: EntityState }),
    type: t.expose('type', { type: EntityType }),
    slug: t.exposeString('slug'),
    permalink: t.exposeString('permalink'),
    order: t.exposeString('order'),
    depth: t.exposeInt('depth'),
    visibility: t.expose('visibility', { type: EntityVisibility }),

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
    user: t.expose('userId', { type: User }),

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

    descendants: t.field({
      type: [Entity],
      resolve: async (self) => {
        const rows = await db.execute<{ id: string }>(sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}, ${Entities.depth}
            FROM ${Entities}
            WHERE ${eq(Entities.id, self.id)}
            UNION ALL
            SELECT ${Entities.id}, ${Entities.depth}
            FROM ${Entities}
            JOIN sq ON ${Entities.parentId} = sq.id
            WHERE ${eq(Entities.state, EntityState.ACTIVE)}
          )
          SELECT id
          FROM sq
          WHERE ${ne(sql`id`, self.id)}
          ORDER BY depth ASC
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
              .where(
                and(
                  inArray(Entities.parentId, ids),
                  eq(Entities.state, EntityState.ACTIVE),
                  eq(Entities.visibility, EntityVisibility.UNLISTED),
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
            JOIN sq ON ${Entities.id} = sq.parent_id
            WHERE sq.parent_id IS NOT NULL AND
            ${eq(Entities.visibility, EntityVisibility.UNLISTED)}
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
      const entity = await db.select().from(Entities).where(eq(Entities.id, args.entityId)).then(firstOrThrowWith(new NotFoundError()));

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      }).catch(() => {
        throw new NotFoundError();
      });

      return entity;
    },
  }),

  entityView: t.field({
    type: EntityView,
    args: { origin: t.arg.string(), slug: t.arg.string() },
    resolve: async (_, args, ctx) => {
      const pattern = new RegExp(`^${escape(env.USERSITE_URL).replace(String.raw`\*\.`, String.raw`([^.]+)\.`)}$`);
      const slug = args.origin.match(pattern)?.[1];
      if (!slug) {
        throw new TypieError({ code: 'invalid_hostname' });
      }

      const site = await db
        .select({ id: Sites.id })
        .from(Sites)
        .where(and(eq(Sites.slug, slug), eq(Sites.state, SiteState.ACTIVE)))
        .then(firstOrThrowWith(new NotFoundError()));

      const entity = await db
        .select()
        .from(Entities)
        .where(and(eq(Entities.siteId, site.id), eq(Entities.slug, args.slug), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrowWith(new NotFoundError()));

      if (entity.visibility === EntityVisibility.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session?.userId,
          siteId: entity.siteId,
        }).catch(() => {
          throw new NotFoundError();
        });
      }

      return entity;
    },
  }),

  permalink: t.field({
    type: t.builder.simpleObject('Permalink', {
      fields: (t) => ({
        siteUrl: t.string(),
        entitySlug: t.string(),
      }),
    }),
    args: { permalink: t.arg.string() },
    resolve: async (_, args) => {
      const entity = await db
        .select({ siteSlug: Sites.slug, entitySlug: Entities.slug })
        .from(Entities)
        .innerJoin(Sites, eq(Entities.siteId, Sites.id))
        .where(and(eq(Entities.permalink, args.permalink), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrowWith(new NotFoundError()));

      return {
        siteUrl: env.USERSITE_URL.replace('*', entity.siteSlug),
        entitySlug: entity.entitySlug,
      };
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
        .select({ id: Entities.id, siteId: Entities.siteId, parentId: Entities.parentId, depth: Entities.depth })
        .from(Entities)
        .where(eq(Entities.id, input.entityId))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      let parentId, depth;

      if (input.parentEntityId === null) {
        parentId = null;
        depth = 0;
      } else if (input.parentEntityId === undefined) {
        parentId = entity.parentId;
        depth = entity.depth;
      } else {
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

        parentId = parentEntity.id;
        depth = parentEntity.depth + 1;
      }

      const depthDelta = depth - entity.depth;

      const updatedEntity = await db.transaction(async (tx) => {
        const updatedEntity = await tx
          .update(Entities)
          .set({
            parentId,
            depth,
            order: generateEntityOrder({
              lower: input.lowerOrder,
              upper: input.upperOrder,
            }),
          })
          .where(eq(Entities.id, entity.id))
          .returning()
          .then(firstOrThrow);

        if (depthDelta !== 0) {
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
        }

        return updatedEntity;
      });

      pubsub.publish('site:update', entity.siteId, { scope: 'site' });

      return updatedEntity;
    },
  }),
}));
