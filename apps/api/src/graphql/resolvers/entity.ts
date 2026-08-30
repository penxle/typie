import { EntityAvailability, EntityState, EntityType, EntityVisibility, NoteState, RedirectType, SiteState } from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, count, desc, eq, gt, inArray, isNull, lt, ne, sql } from 'drizzle-orm';
import escape from 'escape-string-regexp';
import { match } from 'ts-pattern';
import {
  db,
  Dividers,
  DocumentHeads,
  Documents,
  DocumentStates,
  Entities,
  EntityGoals,
  first,
  firstOrThrow,
  firstOrThrowWith,
  Folders,
  NoteEntities,
  Notes,
  Redirects,
  Sites,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { env } from '#/env.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { deleteEntitiesCore, moveEntitiesCore, recoverEntityCore, updateEntityIconCore } from '#/utils/entity-actions.ts';
import { buildDailyHistory } from '#/utils/goal.ts';
import { buildFreshV2Content, copyEntityRecursive, generateFractionalOrder } from '#/utils/index.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { enqueueSearchSyncForEntityIds } from '#/utils/search-index.ts';
import { builder } from '../builder.ts';
import {
  Entity,
  EntityCharacterCountHistory,
  EntityContainer,
  EntityGoal,
  EntityNode,
  EntityView,
  EntityViewNode,
  IEntity,
  isTypeOf,
  Note,
  Site,
  SiteView,
  User,
} from '../objects.ts';
import type { FreshV2Content } from '#/utils/index.ts';

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
    icon: t.exposeString('icon'),
    iconColor: t.exposeString('iconColor'),

    url: t.string({ resolve: (self) => `${env.USERSITE_URL.replace('*.', '')}/${self.permalink}` }),
  }),
});

Entity.implement({
  isTypeOf: isTypeOf(TableCode.ENTITIES),
  interfaces: [IEntity],
  fields: (t) => ({
    view: t.expose('id', { type: EntityView }),
    deletedAt: t.expose('deletedAt', { type: 'DateTime', nullable: true }),

    site: t.expose('siteId', { type: Site }),
    parent: t.expose('parentId', { type: Entity, nullable: true }),
    container: t.field({
      type: EntityContainer,
      resolve: (self) => ({ id: self.parentId ?? self.siteId }) as never,
    }),
    user: t.expose('userId', { type: User }),

    node: t.field({
      type: EntityNode,
      resolve: async (self, _, ctx) => {
        const loader = match(self.type)
          .with(EntityType.DOCUMENT, () =>
            ctx.loader({
              name: 'Entity.node (Document)',
              load: (ids) => db.select().from(Documents).where(inArray(Documents.entityId, ids)),
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
          .with(EntityType.DIVIDER, () =>
            ctx.loader({
              name: 'Entity.node (Divider)',
              load: (ids) => db.select().from(Dividers).where(inArray(Dividers.entityId, ids)),
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
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

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

    firstChild: t.field({
      type: Entity,
      nullable: true,
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

        const loader = ctx.loader({
          name: 'Entity.firstChild',
          many: true,
          load: async (ids) => {
            return await db.execute<{ id: string; parent_id: string }>(sql`
              SELECT id, parent_id FROM (
                SELECT id, parent_id, ROW_NUMBER() OVER (PARTITION BY parent_id ORDER BY "order" ASC) AS rn
                FROM ${Entities}
                WHERE ${inArray(Entities.parentId, ids)}
                AND ${eq(Entities.state, EntityState.ACTIVE)}
              ) sq WHERE rn = 1
            `);
          },
          key: (row) => row.parent_id,
        });

        const rows = await loader.load(self.id);
        return rows[0]?.id ?? null;
      },
    }),

    lastChild: t.field({
      type: Entity,
      nullable: true,
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

        const loader = ctx.loader({
          name: 'Entity.lastChild',
          many: true,
          load: async (ids) => {
            return await db.execute<{ id: string; parent_id: string }>(sql`
              SELECT id, parent_id FROM (
                SELECT id, parent_id, ROW_NUMBER() OVER (PARTITION BY parent_id ORDER BY "order" DESC) AS rn
                FROM ${Entities}
                WHERE ${inArray(Entities.parentId, ids)}
                AND ${eq(Entities.state, EntityState.ACTIVE)}
              ) sq WHERE rn = 1
            `);
          },
          key: (row) => row.parent_id,
        });

        const rows = await loader.load(self.id);
        return rows[0]?.id ?? null;
      },
    }),

    deletedChildren: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

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
                  ne(Entities.type, EntityType.DIVIDER),
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

    goal: t.field({
      type: EntityGoal,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const userId = ctx.session?.userId;
        if (!userId) {
          return null;
        }

        // 공유 문서의 목표는 문서 독자에게 보이지만, 상위 폴더 목표는 사이트 소유자에게만 보인다.
        if (self.type !== EntityType.DOCUMENT) {
          const permissionLoader = ctx.loader({
            name: 'Entity.goal.permission',
            nullable: true,
            load: async (siteIds: string[]) => {
              const allowed: { siteId: string }[] = [];

              for (const siteId of siteIds) {
                try {
                  await assertSitePermission({ userId, siteId });
                  allowed.push({ siteId });
                } catch (err) {
                  if (!(err instanceof TypieError)) {
                    throw err;
                  }
                }
              }

              return allowed;
            },
            key: (row) => row?.siteId,
          });

          if (!(await permissionLoader.load(self.siteId))) {
            return null;
          }
        }

        const loader = ctx.loader({
          name: 'Entity.goal',
          nullable: true,
          load: async (ids: string[]) => {
            return await db.select().from(EntityGoals).where(inArray(EntityGoals.entityId, ids));
          },
          key: (row) => row?.entityId,
        });

        return await loader.load(self.id);
      },
    }),

    characterCountHistory: t.field({
      type: [EntityCharacterCountHistory],
      resolve: async (self) => {
        const startOfTomorrow = dayjs.kst().startOf('day').add(1, 'day');
        const from = startOfTomorrow.subtract(365, 'days');
        const fromDate = from.format('YYYY-MM-DD');

        const subtree = sql`
          WITH RECURSIVE sq AS (
            SELECT ${Entities.id}
            FROM ${Entities}
            WHERE ${eq(Entities.id, self.id)}
            UNION ALL
            SELECT ${Entities.id}
            FROM ${Entities}
            JOIN sq ON ${Entities.parentId} = sq.id
            WHERE ${Entities.state} = ${EntityState.ACTIVE}
          )
        `;

        const baselineRows = await db.execute<{ document_id: string; character_count: number }>(sql`
          ${subtree}
          SELECT DISTINCT ON (dh.document_id)
            dh.document_id,
            dh.character_count
          FROM ${DocumentHeads} dh
          JOIN ${Documents} d ON d.id = dh.document_id
          JOIN sq ON d.entity_id = sq.id
          WHERE dh.bucket < ${from.toISOString()}
          ORDER BY dh.document_id, dh.bucket DESC, dh.seq DESC NULLS LAST
        `);

        const recentRows = await db.execute<{ document_id: string; date: string; character_count: number }>(sql`
          ${subtree}
          SELECT DISTINCT ON (dh.document_id, DATE(dh.bucket AT TIME ZONE 'Asia/Seoul'))
            dh.document_id,
            TO_CHAR(dh.bucket AT TIME ZONE 'Asia/Seoul', 'YYYY-MM-DD') AS date,
            dh.character_count
          FROM ${DocumentHeads} dh
          JOIN ${Documents} d ON d.id = dh.document_id
          JOIN sq ON d.entity_id = sq.id
          WHERE dh.bucket >= ${from.toISOString()}
          ORDER BY dh.document_id, DATE(dh.bucket AT TIME ZONE 'Asia/Seoul'), dh.bucket DESC, dh.seq DESC NULLS LAST
        `);

        const today = dayjs.kst().format('YYYY-MM-DD');
        const history = buildDailyHistory(
          [
            ...baselineRows.map((row) => ({ documentId: row.document_id, date: fromDate, characterCount: Number(row.character_count) })),
            ...recentRows.map((row) => ({ documentId: row.document_id, date: row.date, characterCount: Number(row.character_count) })),
          ],
          today,
        );

        return history.map((point) => ({ date: dayjs.kst(point.date).startOf('day'), characterCount: point.characterCount }));
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
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.siteId });

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

    notes: t.withAuth({ session: true }).field({
      type: [Note],
      resolve: async (self) => {
        const rows = await db
          .select({ note: Notes })
          .from(NoteEntities)
          .innerJoin(Notes, eq(NoteEntities.noteId, Notes.id))
          .where(and(eq(NoteEntities.entityId, self.id), eq(Notes.state, NoteState.ACTIVE)));

        return rows.map((r) => r.note).toSorted((a, b) => a.order.localeCompare(b.order));
      },
    }),

    notesCount: t.int({
      resolve: async (self) => {
        const row = await db
          .select({ count: count() })
          .from(NoteEntities)
          .innerJoin(Notes, eq(NoteEntities.noteId, Notes.id))
          .where(and(eq(NoteEntities.entityId, self.id), eq(Notes.state, NoteState.ACTIVE)))
          .then(firstOrThrow);

        return row.count;
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
          .with(EntityType.DOCUMENT, () =>
            ctx.loader({
              name: 'EntityView.node (Document)',
              load: (ids) => db.select().from(Documents).where(inArray(Documents.entityId, ids)),
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
          .with(EntityType.DIVIDER, () => {
            throw new NotFoundError();
          })
          .exhaustive();

        return await loader.load(self.id);
      },
    }),

    children: t.field({
      type: [EntityView],
      resolve: async (self) => {
        const visibilities =
          self.visibility === EntityVisibility.PUBLIC ? [EntityVisibility.PUBLIC] : [EntityVisibility.PUBLIC, EntityVisibility.UNLISTED];

        return await db
          .select()
          .from(Entities)
          .where(
            and(
              eq(Entities.parentId, self.id),
              eq(Entities.state, EntityState.ACTIVE),
              ne(Entities.type, EntityType.DIVIDER),
              inArray(Entities.visibility, visibilities),
            ),
          )
          .orderBy(asc(Entities.order));
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
            ${inArray(Entities.visibility, [EntityVisibility.UNLISTED, EntityVisibility.PUBLIC])}
          )
          SELECT id
          FROM sq
          WHERE ${ne(sql`id`, self.id)}
          ORDER BY depth DESC
        `);

        return rows.map(({ id }) => id);
      },
    }),

    prev: t.field({
      type: EntityView,
      nullable: true,
      resolve: async (self) => {
        if (self.type !== EntityType.DOCUMENT) return null;

        let visibilities: EntityVisibility[] = [EntityVisibility.PUBLIC];

        if (self.parentId) {
          const parent = await db
            .select({ visibility: Entities.visibility })
            .from(Entities)
            .where(eq(Entities.id, self.parentId))
            .then(first);

          if (parent?.visibility === EntityVisibility.UNLISTED) {
            visibilities = [EntityVisibility.PUBLIC, EntityVisibility.UNLISTED];
          }
        }

        return await db
          .select()
          .from(Entities)
          .where(
            and(
              eq(Entities.siteId, self.siteId),
              self.parentId ? eq(Entities.parentId, self.parentId) : isNull(Entities.parentId),
              eq(Entities.state, EntityState.ACTIVE),
              eq(Entities.type, EntityType.DOCUMENT),
              inArray(Entities.visibility, visibilities),
              lt(Entities.order, self.order),
            ),
          )
          .orderBy(desc(Entities.order))
          .limit(1)
          .then(first);
      },
    }),

    next: t.field({
      type: EntityView,
      nullable: true,
      resolve: async (self) => {
        if (self.type !== EntityType.DOCUMENT) return null;

        let visibilities: EntityVisibility[] = [EntityVisibility.PUBLIC];

        if (self.parentId) {
          const parent = await db
            .select({ visibility: Entities.visibility })
            .from(Entities)
            .where(eq(Entities.id, self.parentId))
            .then(first);

          if (parent?.visibility === EntityVisibility.UNLISTED) {
            visibilities = [EntityVisibility.PUBLIC, EntityVisibility.UNLISTED];
          }
        }

        return await db
          .select()
          .from(Entities)
          .where(
            and(
              eq(Entities.siteId, self.siteId),
              self.parentId ? eq(Entities.parentId, self.parentId) : isNull(Entities.parentId),
              eq(Entities.state, EntityState.ACTIVE),
              eq(Entities.type, EntityType.DOCUMENT),
              inArray(Entities.visibility, visibilities),
              gt(Entities.order, self.order),
            ),
          )
          .orderBy(asc(Entities.order))
          .limit(1)
          .then(first);
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

      const slug = args.entityId ? undefined : args.slug;

      const entity = await db
        .select()
        .from(Entities)
        .where(
          args.entityId
            ? eq(Entities.id, args.entityId)
            : eq(
                Entities.slug,
                sql<string>`COALESCE((SELECT ${Redirects.to} FROM ${Redirects} WHERE ${Redirects.type} = ${RedirectType.SLUG} AND ${Redirects.from} = ${slug as string}), ${slug as string})`,
              ),
        )
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

      const slugs = args.entityIds ? undefined : args.slugs;

      const entities = await db
        .select()
        .from(Entities)
        .where(
          args.entityIds
            ? inArray(Entities.id, args.entityIds)
            : inArray(
                Entities.slug,
                sql`(SELECT COALESCE((SELECT ${Redirects.to} FROM ${Redirects} WHERE ${Redirects.type} = ${RedirectType.SLUG} AND ${Redirects.from} = s.val), s.val) FROM unnest(${slugs as string[]}::text[]) AS s(val))`,
              ),
        );

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
        .where(
          and(
            eq(Entities.siteId, site.id),
            eq(Entities.state, EntityState.ACTIVE),
            ne(Entities.type, EntityType.DIVIDER),
            eq(
              Entities.slug,
              sql<string>`COALESCE((SELECT ${Redirects.to} FROM ${Redirects} WHERE ${Redirects.type} = ${RedirectType.SLUG} AND ${Redirects.from} = ${args.slug}), ${args.slug})`,
            ),
          ),
        )
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
        .where(
          and(
            eq(Entities.state, EntityState.ACTIVE),
            ne(Entities.type, EntityType.DIVIDER),
            eq(
              Entities.permalink,
              sql<string>`COALESCE((SELECT ${Redirects.to} FROM ${Redirects} WHERE ${Redirects.type} = ${RedirectType.PERMALINK} AND ${Redirects.from} = ${args.permalink}), ${args.permalink})`,
            ),
          ),
        )
        .then(firstOrThrowWith(new NotFoundError()));

      return {
        siteUrl: env.USERSITE_URL.replace('*', () => entity.siteSlug),
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

      await assertActiveSubscription({ userId: ctx.session.userId });

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
            order: generateFractionalOrder({
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

  updateEntityIcon: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      icon: t.input.string(),
      iconColor: t.input.string(),
    },
    resolve: async (_, { input }, ctx) =>
      await updateEntityIconCore(db, {
        userId: ctx.session.userId,
        entityId: input.entityId,
        icon: input.icon,
        iconColor: input.iconColor,
      }),
  }),

  updateEntitiesIcon: t.withAuth({ session: true }).fieldWithInput({
    type: [Entity],
    input: {
      entityIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }),
      icon: t.input.string({ required: false }),
      iconColor: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      if (!input.icon && !input.iconColor) {
        return [];
      }

      const entities = await db
        .select({ id: Entities.id, siteId: Entities.siteId, parentId: Entities.parentId })
        .from(Entities)
        .where(and(inArray(Entities.id, input.entityIds), eq(Entities.state, EntityState.ACTIVE)));

      if (entities.length === 0) {
        return [];
      }

      const siteId = entities[0].siteId;

      await assertSitePermission({
        userId: ctx.session.userId,
        siteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

      if (entities.some((entity) => entity.siteId !== siteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      const set: Record<string, unknown> = {};
      if (input.icon) {
        set.icon = input.icon;
      }
      if (input.iconColor) {
        set.iconColor = input.iconColor;
      }

      const entityIds = entities.map((e) => e.id);
      const updatedEntities = await db.update(Entities).set(set).where(inArray(Entities.id, entityIds)).returning();

      const parentIds = new Set(entities.map((e) => e.parentId).filter((id): id is string => id !== null));
      for (const parentId of parentIds) {
        pubsub.publish('site:update', siteId, { scope: 'entity', entityId: parentId });
      }
      if (entities.some((e) => e.parentId === null)) {
        pubsub.publish('site:update', siteId, { scope: 'site' });
      }

      return updatedEntities;
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
      targetSiteId: t.input.id({ validate: validateDbId(TableCode.SITES), required: false }),
    },
    resolve: async (_, { input }, ctx) =>
      await moveEntitiesCore(db, {
        userId: ctx.session.userId,
        entityIds: input.entityIds,
        parentEntityId: input.parentEntityId ?? null,
        lowerOrder: input.lowerOrder ?? null,
        upperOrder: input.upperOrder ?? null,
        targetSiteId: input.targetSiteId ?? null,
      }),
  }),

  deleteEntities: t.withAuth({ session: true }).fieldWithInput({
    type: [Entity],
    input: { entityIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }) },
    resolve: async (_, { input }, ctx) => await deleteEntitiesCore(db, { userId: ctx.session.userId, entityIds: input.entityIds }),
  }),

  copyEntities: t.withAuth({ session: true }).fieldWithInput({
    type: [Entity],
    input: {
      entityIds: t.input.idList({ validate: { items: validateDbId(TableCode.ENTITIES) } }),
      targetSiteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      parentEntityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES), required: false }),
      lowerOrder: t.input.string({ required: false }),
      upperOrder: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      // 자손 필터링 (moveEntities 패턴)
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
        ORDER BY ${Entities.order} ASC
      `);

      if (entities.length === 0) {
        throw new TypieError({ code: 'paste_source_not_found' });
      }

      // 원본 사이트 권한 확인
      const sourceSiteId = entities[0].site_id;
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: sourceSiteId,
      });

      if (entities.some((entity) => entity.site_id !== sourceSiteId)) {
        throw new TypieError({ code: 'site_mismatch' });
      }

      // 대상 사이트 권한 확인
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.targetSiteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

      // 대상 depth 계산
      let targetDepth = 0;
      if (input.parentEntityId) {
        const parentEntity = await db
          .select({ depth: Entities.depth, siteId: Entities.siteId })
          .from(Entities)
          .where(and(eq(Entities.id, input.parentEntityId), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrowWith(new NotFoundError()));

        if (parentEntity.siteId !== input.targetSiteId) {
          throw new TypieError({ code: 'site_mismatch' });
        }

        if (input.entityIds.includes(input.parentEntityId)) {
          throw new TypieError({ code: 'circular_reference' });
        }

        if (sourceSiteId === input.targetSiteId) {
          const [hasCycle] = await db.execute<{ exists: boolean }>(
            sql`
              WITH RECURSIVE sq AS (
                SELECT ${Entities.id}, ${Entities.parentId}
                FROM ${Entities}
                WHERE ${eq(Entities.id, input.parentEntityId)}
                UNION ALL
                SELECT ${Entities.id}, ${Entities.parentId}
                FROM ${Entities}
                JOIN sq ON ${Entities.id} = sq.parent_id
              )
              SELECT EXISTS (
                SELECT 1 FROM sq WHERE ${inArray(
                  sql`id`,
                  entities.map((entity) => entity.id),
                )}
              ) as exists
            `,
          );

          if (hasCycle.exists) {
            throw new TypieError({ code: 'circular_reference' });
          }
        }

        targetDepth = parentEntity.depth + 1;
      }

      const descendantV2Docs = await db.execute<{ document_id: string }>(sql`
        WITH RECURSIVE tree AS (
          SELECT ${Entities.id}
          FROM ${Entities}
          WHERE ${inArray(
            Entities.id,
            entities.map((entity) => entity.id),
          )}
          UNION ALL
          SELECT ${Entities.id}
          FROM ${Entities}
          JOIN tree ON ${Entities.parentId} = tree.id
          WHERE ${eq(Entities.state, EntityState.ACTIVE)}
        )
        SELECT ${Documents.id} AS document_id
        FROM tree
        JOIN ${Documents} ON ${Documents.entityId} = tree.id
        JOIN ${DocumentStates} ON ${DocumentStates.documentId} = ${Documents.id}
      `);

      const v2Map = new Map<string, FreshV2Content>();
      for (const row of descendantV2Docs) {
        const content = await buildFreshV2Content(row.document_id);
        if (content) {
          v2Map.set(row.document_id, content);
        }
      }

      const newEntityIds = await db.transaction(async (tx) => {
        const ids: string[] = [];
        let lastOrder = input.lowerOrder ?? null;

        for (const entity of entities) {
          const order = generateFractionalOrder({
            lower: lastOrder,
            upper: input.upperOrder ?? null,
          });
          const newId = await copyEntityRecursive(
            tx,
            entity.id,
            input.targetSiteId,
            input.parentEntityId ?? null,
            targetDepth,
            order,
            ctx.session.userId,
            v2Map,
          );
          ids.push(newId);
          lastOrder = order;
        }

        return ids;
      });

      // Pubsub 발행
      if (input.parentEntityId) {
        pubsub.publish('site:update', input.targetSiteId, { scope: 'entity', entityId: input.parentEntityId });
      } else {
        pubsub.publish('site:update', input.targetSiteId, { scope: 'site' });
      }
      pubsub.publish('user:usage:update', ctx.session.userId, null);

      // 검색 인덱스 enqueue
      const newDocuments = await db
        .select({ id: Documents.id })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(inArray(Entities.id, newEntityIds));

      for (const doc of newDocuments) {
        await enqueueJob('search:index:document', doc.id);
      }

      // 새 엔티티 반환
      return await db.select().from(Entities).where(inArray(Entities.id, newEntityIds));
    },
  }),

  recoverEntity: t.withAuth({ session: true }).fieldWithInput({
    type: Entity,
    input: { entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }) },
    resolve: async (_, { input }, ctx) => await recoverEntityCore(db, { userId: ctx.session.userId, entityId: input.entityId }),
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

      await enqueueSearchSyncForEntityIds(input.entityIds);

      pubsub.publish('site:update', siteId, { scope: 'site' });

      return siteId;
    },
  }),
}));
