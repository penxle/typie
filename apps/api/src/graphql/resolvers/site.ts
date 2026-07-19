import { faker } from '@faker-js/faker';
import { DocumentType, EntityState, EntityType, EntityVisibility, SiteDateDisplay, SiteState } from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import { siteSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { and, asc, desc, eq, getTableColumns, gt, inArray, isNull, ne, or, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import escape from 'escape-string-regexp';
import { match } from 'ts-pattern';
import { clearLoaders } from '#/context.ts';
import { db, Documents, Entities, first, firstOrThrow, firstOrThrowWith, Sites, TableCode, Users, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { pubsub } from '#/pubsub.ts';
import { generateRandomAvatar, persistBlobAsImage } from '#/utils/index.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { builder } from '../builder.ts';
import { Document, Entity, EntityView, Image, ISite, isTypeOf, Post, Site, SiteView, User } from '../objects.ts';

/**
 * * Types
 */

ISite.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),
    logo: t.expose('logoId', { type: Image }),

    dateDisplay: t.expose('dateDisplay', { type: SiteDateDisplay }),

    url: t.string({ resolve: (self) => env.USERSITE_URL.replace('*', () => self.slug) }),
  }),
});

Site.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  interfaces: [ISite],
  fields: (t) => ({
    view: t.expose('id', { type: SiteView }),
    user: t.expose('userId', { type: User }),

    entities: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Site.entities',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Entities)
              .where(
                and(
                  inArray(Entities.siteId, ids),
                  eq(Entities.state, EntityState.ACTIVE),
                  ne(Entities.type, EntityType.POST),
                  isNull(Entities.parentId),
                ),
              )
              .orderBy(asc(Entities.order));
          },
          key: ({ siteId }) => siteId,
        });

        return await loader.load(self.id);
      },
    }),

    firstEntity: t.field({
      type: Entity,
      nullable: true,
      args: { type: t.arg({ type: EntityType }) },
      resolve: async (self, args) => {
        const rows = await db.execute<{ id: string }>(
          sql`
            WITH RECURSIVE sq AS (
              SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.type}, ${Entities.order}, ${Entities.state}, ARRAY[${Entities.order}] as path_array, 1 as depth
              FROM ${Entities}
              WHERE ${and(eq(Entities.siteId, self.id), isNull(Entities.parentId), eq(Entities.state, EntityState.ACTIVE))}
              UNION ALL
              SELECT ${Entities.id}, ${Entities.parentId}, ${Entities.type}, ${Entities.order}, ${Entities.state}, sq.path_array || ${Entities.order}, sq.depth + 1
              FROM ${Entities}
              JOIN sq ON ${Entities.parentId} = sq.id
              WHERE ${eq(Entities.state, EntityState.ACTIVE)}
            )
            SELECT sq.id
            FROM sq
            WHERE sq.type = ${args.type}
            ORDER BY sq.path_array
            LIMIT 1;
          `,
        );

        return rows[0]?.id;
      },
    }),

    lastRootEntity: t.field({
      type: Entity,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'Site.lastRootEntity',
          many: true,
          load: async (ids) => {
            return await db.execute<{ id: string; site_id: string }>(sql`
              SELECT id, site_id FROM (
                SELECT id, site_id, ROW_NUMBER() OVER (PARTITION BY site_id ORDER BY "order" DESC) AS rn
                FROM ${Entities}
                WHERE ${inArray(Entities.siteId, ids)}
                AND ${eq(Entities.state, EntityState.ACTIVE)}
                AND ${ne(Entities.type, EntityType.POST)}
                AND ${isNull(Entities.parentId)}
              ) sq WHERE rn = 1
            `);
          },
          key: (row) => row.site_id,
        });

        const rows = await loader.load(self.id);
        return rows[0]?.id ?? null;
      },
    }),

    templates: t.field({
      type: [Post],
      resolve: async () => [],
    }),

    documentTemplates: t.field({
      type: [Document],
      resolve: async (self) => {
        return await db
          .select(getTableColumns(Documents))
          .from(Documents)
          .innerJoin(Entities, eq(Documents.entityId, Entities.id))
          .where(and(eq(Entities.siteId, self.id), eq(Documents.type, DocumentType.TEMPLATE), eq(Entities.state, EntityState.ACTIVE)))
          .orderBy(asc(Documents.createdAt));
      },
    }),

    folderCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            SELECT COUNT(*) AS count
            FROM ${Entities}
            WHERE site_id = ${self.id}
            AND state = ${EntityState.ACTIVE}
            AND type = ${EntityType.FOLDER}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    documentCount: t.int({
      resolve: async (self) => {
        const rows = await db.execute<{ count: number }>(
          sql`
            SELECT COUNT(*) AS count
            FROM ${Entities}
            WHERE site_id = ${self.id}
            AND state = ${EntityState.ACTIVE}
            AND type = ${EntityType.DOCUMENT}
          `,
        );
        return Number(rows[0]?.count || 0);
      },
    }),

    deletedEntities: t.field({
      type: [Entity],
      resolve: async (self) => {
        const parentEntities = alias(Entities, 'parent_entities');
        return await db
          .select(getTableColumns(Entities))
          .from(Entities)
          .leftJoin(parentEntities, eq(Entities.parentId, parentEntities.id))
          .where(
            and(
              eq(Entities.siteId, self.id),
              eq(Entities.state, EntityState.DELETED),
              ne(Entities.type, EntityType.POST),
              gt(Entities.deletedAt, dayjs().subtract(30, 'days')),
              or(isNull(parentEntities.id), eq(parentEntities.state, EntityState.ACTIVE)),
            ),
          )
          .orderBy(desc(Entities.deletedAt));
      },
    }),
  }),
});

SiteView.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  interfaces: [ISite],
  fields: (t) => ({
    entities: t.field({
      type: [EntityView],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'SiteView.entities',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Entities)
              .where(
                and(
                  inArray(Entities.siteId, ids),
                  eq(Entities.state, EntityState.ACTIVE),
                  ne(Entities.type, EntityType.POST),
                  eq(Entities.visibility, EntityVisibility.PUBLIC),
                  isNull(Entities.parentId),
                ),
              )
              .orderBy(asc(Entities.order));
          },
          key: ({ siteId }) => siteId,
        });

        return await loader.load(self.id);
      },
    }),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  site: t.withAuth({ session: true }).field({
    type: Site,
    args: { siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }) },
    resolve: async (_, args, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: args.siteId,
      });

      return args.siteId;
    },
  }),

  siteView: t.field({
    type: SiteView,
    args: { origin: t.arg.string() },
    resolve: async (_, args) => {
      const pattern = new RegExp(`^${escape(env.USERSITE_URL).replace(String.raw`\*\.`, String.raw`([^.]+)\.`)}$`);
      const slug = args.origin.match(pattern)?.[1];
      if (!slug) {
        throw new TypieError({ code: 'invalid_hostname' });
      }

      const site = await db
        .select()
        .from(Sites)
        .where(and(eq(Sites.slug, slug), eq(Sites.state, SiteState.ACTIVE)))
        .then(firstOrThrowWith(new NotFoundError()));

      return site;
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  updateSite: t.withAuth({ session: true }).fieldWithInput({
    type: Site,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      name: t.input.string({ required: false }),
      logoId: t.input.id({ required: false }),
      dateDisplay: t.input.field({ type: SiteDateDisplay, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

      const updateData: { name?: string; logoId?: string; dateDisplay?: SiteDateDisplay } = {};
      if (input.name !== undefined && input.name !== null) {
        updateData.name = input.name;
      }
      if (input.logoId !== undefined && input.logoId !== null) {
        updateData.logoId = input.logoId;
      }
      if (input.dateDisplay !== undefined && input.dateDisplay !== null) {
        updateData.dateDisplay = input.dateDisplay;
      }

      if (Object.keys(updateData).length === 0) {
        return await db.select().from(Sites).where(eq(Sites.id, input.siteId)).then(firstOrThrow);
      }

      return await db.update(Sites).set(updateData).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
    },
  }),

  updateSiteSlug: t.withAuth({ session: true }).fieldWithInput({
    type: Site,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
      slug: t.input.string({ validate: { schema: siteSchema.slug } }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

      const slugExistSite = await db
        .select({ id: Sites.id })
        .from(Sites)
        .where(and(eq(Sites.slug, input.slug), ne(Sites.id, input.siteId)))
        .then(first);

      if (slugExistSite) {
        throw new TypieError({ code: 'site_slug_already_exists' });
      }

      return await db.update(Sites).set({ slug: input.slug }).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
    },
  }),

  createSite: t.withAuth({ session: true }).fieldWithInput({
    type: Site,
    input: {
      name: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      await assertActiveSubscription({ userId: ctx.session.userId });

      const logoFile = await generateRandomAvatar();
      const logo = await persistBlobAsImage({ file: logoFile });

      const slug = [
        faker.word.adjective({ length: { min: 3, max: 5 } }),
        faker.word.noun({ length: { min: 4, max: 6 } }),
        faker.string.numeric({ length: { min: 3, max: 4 } }),
      ].join('-');

      const site = await db
        .insert(Sites)
        .values({
          userId: ctx.session.userId,
          slug,
          name: input.name,
          logoId: logo.id,
        })
        .returning()
        .then(firstOrThrow);

      return site;
    },
  }),

  deleteSite: t.withAuth({ session: true }).fieldWithInput({
    type: Site,
    input: {
      siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      return await db.transaction(async (tx) => {
        const activeSites = await tx
          .select({ id: Sites.id })
          .from(Sites)
          .where(and(eq(Sites.userId, ctx.session.userId), eq(Sites.state, SiteState.ACTIVE)))
          .for('update');

        if (activeSites.length <= 1) {
          throw new TypieError({ code: 'cannot_delete_last_site' });
        }

        return await tx.update(Sites).set({ state: SiteState.DELETED }).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
      });
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  siteUpdateStream: t.withAuth({ session: true }).field({
    type: t.builder.unionType('SiteUpdateStreamPayload', {
      types: [Site, Entity],
    }),
    args: { siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }) },
    subscribe: async (_, args, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: args.siteId,
      });

      const repeater = pubsub.subscribe('site:update', args.siteId);

      ctx.c.req.raw.signal.addEventListener('abort', () => {
        repeater.return();
      });

      return repeater;
    },
    resolve: async (payload, args, ctx) => {
      clearLoaders(ctx);

      return match(payload)
        .with({ scope: 'site' }, () => db.select().from(Sites).where(eq(Sites.id, args.siteId)).then(firstOrThrow))
        .with({ scope: 'entity' }, ({ entityId }) => db.select().from(Entities).where(eq(Entities.id, entityId)).then(firstOrThrow))
        .exhaustive();
    },
  }),

  siteUsageUpdateStream: t.withAuth({ session: true }).field({
    type: Site,
    args: { siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }) },
    subscribe: async (_, args, ctx) => {
      // await assertSitePermission({
      //   userId: ctx.session.userId,
      //   siteId: args.siteId,
      // });

      const repeater = pubsub.subscribe('site:usage:update', args.siteId);

      ctx.c.req.raw.signal.addEventListener('abort', () => {
        repeater.return();
      });

      return repeater;
    },
    resolve: async (_, args, ctx) => {
      clearLoaders(ctx);

      return await db.select().from(Sites).where(eq(Sites.id, args.siteId)).then(firstOrThrow);
    },
  }),

  userUsageUpdateStream: t.withAuth({ session: true }).field({
    type: User,
    args: { userId: t.arg.id({ validate: validateDbId(TableCode.USERS) }) },
    subscribe: async (_, args, ctx) => {
      if (ctx.session.userId !== args.userId) {
        throw new TypieError({ code: 'permission_denied' });
      }

      const repeater = pubsub.subscribe('user:usage:update', args.userId);

      ctx.c.req.raw.signal.addEventListener('abort', () => {
        repeater.return();
      });

      return repeater;
    },
    resolve: async (_, args, ctx) => {
      clearLoaders(ctx);

      return await db.select().from(Users).where(eq(Users.id, args.userId)).then(firstOrThrow);
    },
  }),
}));
