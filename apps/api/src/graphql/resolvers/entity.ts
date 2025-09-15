import dayjs from 'dayjs';
import { and, asc, desc, eq, getTableColumns, gt, inArray, isNull, ne, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import escape from 'escape-string-regexp';
import { match } from 'ts-pattern';
import { Canvases, db, Entities, first, firstOrThrow, firstOrThrowWith, Folders, Posts, Sites, TableCode, validateDbId } from '@/db';
import { EntityAvailability, EntityState, EntityType, EntityVisibility, SiteState } from '@/enums';
import { env } from '@/env';
import { NotFoundError, TypieError } from '@/errors';
import { enqueueJob } from '@/mq';
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
    availability: t.expose('availability', { type: EntityAvailability }),

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
          .with(EntityType.CANVAS, () =>
            ctx.loader({
              name: 'Entity.node (Canvas)',
              load: (ids) => db.select().from(Canvases).where(inArray(Canvases.entityId, ids)),
              key: ({ entityId }) => entityId,
            }),
          )
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

    deletedChildren: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Entity.deletedChildren',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Entities)
              .where(
                and(
                  inArray(Entities.parentId, ids),
                  eq(Entities.state, EntityState.DELETED),
                  gt(Entities.deletedAt, dayjs().subtract(30, 'days')),
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
          .with(EntityType.CANVAS, () =>
            ctx.loader({
              name: 'EntityView.node (Canvas)',
              load: (ids) => db.select().from(Canvases).where(inArray(Canvases.entityId, ids)),
              key: ({ entityId }) => entityId,
            }),
          )
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
    args: {
      entityId: t.arg.id({ validate: validateDbId(TableCode.ENTITIES), required: false }),
      slug: t.arg.string({ required: false }),
    },
    resolve: async (_, args, ctx) => {
      if (!args.entityId && !args.slug) {
        throw new TypieError({ code: 'invalid_argument' });
      }

      const entity = await db
        .select()
        .from(Entities)
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        .where(args.entityId ? eq(Entities.id, args.entityId) : eq(Entities.slug, args.slug!))
        .then(firstOrThrowWith(new NotFoundError()));

      if (entity.availability === EntityAvailability.PRIVATE) {
        await assertSitePermission({
          userId: ctx.session.userId,
          siteId: entity.siteId,
        }).catch(() => {
          throw new NotFoundError();
        });
      }

      return entity;
    },
  }),

  entities: t.withAuth({ session: true }).field({
    type: [Entity],
    args: {
      entityIds: t.arg.idList({ required: false, validate: { items: validateDbId(TableCode.ENTITIES) } }),
      slugs: t.arg.stringList({ required: false }),
    },
    resolve: async (_, args, ctx) => {
      if (!args.entityIds && !args.slugs) {
        throw new TypieError({ code: 'invalid_argument' });
      }

      const entities = await db
        .select()
        .from(Entities)
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        .where(args.entityIds ? inArray(Entities.id, args.entityIds) : inArray(Entities.slug, args.slugs!));

      if (entities.length === 0) {
        return [];
      }

      const privateEntities = entities.filter((entity) => entity.availability === EntityAvailability.PRIVATE);
      const privateSiteIds = [...new Set(privateEntities.map((entity) => entity.siteId))];

      await Promise.all(
        privateSiteIds.map((siteId) =>
          assertSitePermission({
            userId: ctx.session.userId,
            siteId,
          }).catch(() => {
            throw new NotFoundError();
          }),
        ),
      );

      return entities;
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
      treatEmptyParentIdAsRoot: t.input.boolean({ required: false, defaultValue: false }),
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

      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ id: Entities.id, depth: Entities.depth })
          .from(Entities)
          .where(and(eq(Entities.id, input.parentEntityId), eq(Entities.siteId, entity.siteId)))
          .then(firstOrThrow);

        const [hasCycle] = await db.execute<{ exists: boolean }>(
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
        );

        if (hasCycle.exists) {
          throw new TypieError({ code: 'circular_reference' });
        }

        parentId = parentEntity.id;
        depth = parentEntity.depth + 1;
      } else {
        if (input.treatEmptyParentIdAsRoot) {
          parentId = null;
          depth = 0;
        } else {
          if (input.parentEntityId === null) {
            parentId = null;
            depth = 0;
          } else {
            parentId = entity.parentId;
            depth = entity.depth;
          }
        }
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

  viewEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: { entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }) },
    resolve: async (_, { input }, ctx) => {
      const entity = await db
        .select({ userId: Entities.userId })
        .from(Entities)
        .where(and(eq(Entities.id, input.entityId), eq(Entities.state, EntityState.ACTIVE)))
        .then(firstOrThrowWith(new NotFoundError()));

      if (entity.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'forbidden' });
      }

      await db.update(Entities).set({ viewedAt: dayjs() }).where(eq(Entities.id, input.entityId));

      return input.entityId;
    },
  }),

  moveEntities: t.withAuth({ session: true }).fieldWithInput({
    type: [Entity],
    input: {
      entityIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }),
      parentEntityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES), required: false }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const entities = await db.execute<{ id: string; site_id: string; depth: number }>(sql`
        WITH RECURSIVE descendants AS (
          SELECT ${Entities.id}
          FROM ${Entities}
          WHERE ${inArray(Entities.parentId, input.entityIds)}
          UNION ALL
          SELECT ${Entities.id}
          FROM ${Entities}
          JOIN descendants ON ${Entities.parentId} = descendants.id
        )
        SELECT ${Entities.id}, ${Entities.siteId}, ${Entities.depth}
        FROM ${Entities}
        WHERE ${inArray(Entities.id, input.entityIds)}
        AND ${eq(Entities.state, EntityState.ACTIVE)}
        AND ${Entities.id} NOT IN (SELECT id FROM descendants)
      `);

      if (entities.length === 0) {
        return [];
      }

      const siteId = entities[0].site_id;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      if (entities.some((entity) => entity.site_id !== siteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      const targetParentId: string | null = input.parentEntityId ?? null;
      let targetDepth = 0;

      if (targetParentId) {
        const parentEntity = await db
          .select({ depth: Entities.depth, siteId: Entities.siteId })
          .from(Entities)
          .where(and(eq(Entities.id, targetParentId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrowWith(new NotFoundError()));

        if (parentEntity.siteId !== siteId) {
          throw new TypieError({ code: 'site_mismatch' });
        }

        if (input.entityIds.includes(targetParentId)) {
          throw new TypieError({ code: 'circular_reference' });
        }

        const [hasCycle] = await db.execute<{ exists: boolean }>(
          sql`
            WITH RECURSIVE sq AS (
              SELECT ${Entities.id}, ${Entities.parentId}
              FROM ${Entities}
              WHERE ${eq(Entities.id, targetParentId)}
              UNION ALL
              SELECT ${Entities.id}, ${Entities.parentId}
              FROM ${Entities}
              JOIN sq ON ${Entities.id} = sq.parent_id
            )
            SELECT EXISTS (
              SELECT 1 FROM sq WHERE ${inArray(sql`id`, input.entityIds)}
            ) as exists
          `,
        );

        if (hasCycle.exists) {
          throw new TypieError({ code: 'circular_reference' });
        }

        targetDepth = parentEntity.depth + 1;
      }

      return await db.transaction(async (tx) => {
        const movedEntities: (typeof Entities.$inferSelect | string)[] = [];

        let lastOrder = input.lowerOrder ?? null;

        for (const entity of entities) {
          const depthDelta = targetDepth - entity.depth;

          const order = generateEntityOrder({
            lower: lastOrder,
            upper: input.upperOrder ?? null,
          });

          const movedEntity = await tx
            .update(Entities)
            .set({
              parentId: targetParentId,
              depth: targetDepth,
              order,
            })
            .where(eq(Entities.id, entity.id))
            .returning()
            .then(firstOrThrow);

          movedEntities.push(movedEntity);

          lastOrder = order;

          if (depthDelta !== 0) {
            movedEntities.push(
              ...(await tx
                .execute<{ id: string }>(
                  sql`
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
                    RETURNING ${Entities.id}
                  `,
                )
                .then((result) => result.map(({ id }) => id))),
            );
          }
        }

        pubsub.publish('site:update', siteId, { scope: 'site' });

        return movedEntities;
      });
    },
  }),

  deleteEntities: t.withAuth({ session: true }).fieldWithInput({
    type: [Entity],
    input: { entityIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }) },
    resolve: async (_, { input }, ctx) => {
      const entities = await db.execute<{ id: string; site_id: string }>(sql`
        WITH RECURSIVE sq AS (
          SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.siteId}
          FROM ${Entities}
          WHERE ${inArray(Entities.id, input.entityIds)}
          UNION ALL
          SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.siteId}
          FROM ${Entities}
          JOIN sq ON ${Entities.parentId} = sq.id
        )
        SELECT id, site_id
        FROM sq
      `);

      if (entities.length === 0) {
        return [];
      }

      const siteId = entities[0].site_id;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      if (entities.some((entity) => entity.site_id !== siteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      return await db.transaction(async (tx) => {
        const deletedEntities = await tx
          .update(Entities)
          .set({
            state: EntityState.DELETED,
            deletedAt: dayjs(),
          })
          .where(
            inArray(
              Entities.id,
              entities.map(({ id }) => id),
            ),
          )
          .returning();

        pubsub.publish('site:update', siteId, { scope: 'site' });
        pubsub.publish('site:usage:update', siteId, null);

        const postEntityIds: string[] = [];
        const canvasEntityIds: string[] = [];

        for (const entity of deletedEntities) {
          pubsub.publish('site:update', siteId, { scope: 'entity', entityId: entity.id });
          if (entity.type === EntityType.POST) {
            postEntityIds.push(entity.id);
          } else if (entity.type === EntityType.CANVAS) {
            canvasEntityIds.push(entity.id);
          }
        }

        if (postEntityIds.length > 0) {
          const posts = await tx.select({ id: Posts.id }).from(Posts).where(inArray(Posts.entityId, postEntityIds));
          for (const post of posts) {
            await enqueueJob('post:index', post.id);
          }
        }

        if (canvasEntityIds.length > 0) {
          const canvases = await tx.select({ id: Canvases.id }).from(Canvases).where(inArray(Canvases.entityId, canvasEntityIds));
          for (const canvas of canvases) {
            await enqueueJob('canvas:index', canvas.id);
          }
        }

        return deletedEntities;
      });
    },
  }),

  recoverEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: { entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }) },
    resolve: async (_, { input }, ctx) => {
      const ParentEntities = alias(Entities, 'parent_entities');

      const entity = await db
        .select({
          id: Entities.id,
          siteId: Entities.siteId,
          order: Entities.order,
          depth: Entities.depth,
          parentEntity: {
            id: ParentEntities.id,
            state: ParentEntities.state,
            depth: ParentEntities.depth,
          },
        })
        .from(Entities)
        .leftJoin(ParentEntities, eq(Entities.parentId, ParentEntities.id))
        .where(and(eq(Entities.id, input.entityId), eq(Entities.state, EntityState.DELETED)))
        .then(firstOrThrow);

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: entity.siteId,
      });

      const isParentActive = entity.parentEntity?.state === EntityState.ACTIVE;

      const rootLastChildOrder = isParentActive
        ? null
        : await db
            .select({ order: Entities.order })
            .from(Entities)
            .where(and(eq(Entities.siteId, entity.siteId), eq(Entities.state, EntityState.ACTIVE), isNull(Entities.parentId)))
            .orderBy(desc(Entities.order))
            .limit(1)
            .then(first)
            .then((result) => result?.order ?? null);

      const depthDelta = isParentActive ? 0 : -entity.depth;

      return await db.transaction(async (tx) => {
        if (!isParentActive) {
          await tx
            .update(Entities)
            .set({
              parentId: null,
              order: generateEntityOrder({ lower: rootLastChildOrder, upper: null }),
            })
            .where(eq(Entities.id, entity.id));
        }

        const updatedEntities = await tx.execute<{ id: string; type: EntityType }>(
          sql`
            WITH RECURSIVE sq AS (
              SELECT ${Entities.id}
              FROM ${Entities}
              WHERE ${eq(Entities.id, entity.id)}
              UNION ALL
              SELECT ${Entities.id}
              FROM ${Entities}
              JOIN sq ON ${Entities.parentId} = sq.id
            )
            UPDATE ${Entities}
            SET state = ${EntityState.ACTIVE},
            deleted_at = null,
            depth = depth + ${depthDelta}
            WHERE id IN (SELECT id FROM sq) AND ${gt(Entities.deletedAt, dayjs().subtract(30, 'days'))}
            RETURNING ${Entities.id}, ${Entities.type}
          `,
        );

        pubsub.publish('site:update', entity.siteId, { scope: 'site' });
        pubsub.publish('site:usage:update', entity.siteId, null);

        const postEntityIds = updatedEntities.filter(({ type }) => type === EntityType.POST).map(({ id }) => id);
        if (postEntityIds.length > 0) {
          const posts = await tx.select({ id: Posts.id }).from(Posts).where(inArray(Posts.entityId, postEntityIds));
          for (const post of posts) {
            await enqueueJob('post:index', post.id);
          }
        }

        const canvasEntityIds = updatedEntities.filter(({ type }) => type === EntityType.CANVAS).map(({ id }) => id);
        if (canvasEntityIds.length > 0) {
          const canvases = await tx.select({ id: Canvases.id }).from(Canvases).where(inArray(Canvases.entityId, canvasEntityIds));
          for (const canvas of canvases) {
            await enqueueJob('canvas:index', canvas.id);
          }
        }

        return entity.id;
      });
    },
  }),

  purgeEntities: t.withAuth({ session: true }).fieldWithInput({
    type: Site,
    input: { entityIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }) },
    resolve: async (_, { input }, ctx) => {
      const entities = await db.select().from(Entities).where(inArray(Entities.id, input.entityIds));

      if (entities.length === 0) {
        throw new TypieError({ code: 'invalid_argument' });
      }

      const siteId = entities[0].siteId;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      if (entities.some((entity) => entity.state !== EntityState.DELETED || entity.siteId !== siteId)) {
        throw new TypieError({ code: 'invalid_state' });
      }

      await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({
            state: EntityState.PURGED,
            purgedAt: dayjs(),
          })
          .where(inArray(Entities.id, input.entityIds));
      });

      pubsub.publish('site:update', siteId, { scope: 'site' });

      return siteId;
    },
  }),
}));
