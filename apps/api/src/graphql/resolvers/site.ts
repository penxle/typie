import { faker } from '@faker-js/faker';
import { DocumentType, EntityState, EntityType, SiteDateDisplay, SiteState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { siteSchema } from '@typie/lib/validation';
import dayjs from 'dayjs';
import { and, asc, desc, eq, getTableColumns, gt, inArray, isNull, ne, or, sql } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import { match } from 'ts-pattern';
import { clearLoaders } from '#/context.ts';
import { db, Documents, Entities, first, firstOrThrow, Sites, TableCode, Users, validateDbId } from '#/db/index.ts';
import { env } from '#/env.ts';
import { pubsub } from '#/pubsub.ts';
import { enqueueDiscoveryPublicationSync, enqueueDiscoverySiteSync } from '#/utils/discovery-index.ts';
import { generateRandomAvatar, persistBlobAsImage } from '#/utils/index.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { buildPinnedEntitiesBatchQuery } from '#/utils/pinned-entities.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { unpublishBySiteIdsCore } from '#/utils/publication-unpublish.ts';
import {
  buildRecentDocumentsBatchQuery,
  clampRecentDocumentLimit,
  RECENT_DOCUMENT_DEFAULT_LIMIT,
  RECENT_DOCUMENT_SORTS,
  toRecentDocumentsPage,
} from '#/utils/recent-documents.ts';
import { normalizeSiteLinks } from '#/utils/site-core.ts';
import { getTagSuggestions } from '#/utils/tag-suggest.ts';
import { siteUrl } from '#/utils/usersite-core.ts';
import { builder } from '../builder.ts';
import { Document, Entity, Image, ISite, isTypeOf, Site, SiteView, User } from '../objects.ts';
import type { SiteLink } from '#/utils/site-core.ts';

const RecentDocumentSort = builder.enumType('RecentDocumentSort', { values: RECENT_DOCUMENT_SORTS });

const RecentDocumentsResult = builder.simpleObject('RecentDocumentsResult', {
  fields: (t) => ({
    documents: t.field({ type: [Document] }),
    hasMore: t.boolean(),
  }),
});

const TagSuggestion = builder.simpleObject('TagSuggestion', {
  fields: (t) => ({
    name: t.string(),
    count: t.int(),
  }),
});

const TagSuggestions = builder.simpleObject('TagSuggestions', {
  fields: (t) => ({
    mine: t.field({ type: [TagSuggestion] }),
    popular: t.field({ type: [TagSuggestion] }),
  }),
});

/**
 * * Types
 */

const SiteLinkObject = builder.simpleObject('SiteLink', {
  fields: (t) => ({
    label: t.string(),
    url: t.string(),
  }),
});

const SiteLinkInput = builder.inputType('SiteLinkInput', {
  fields: (t) => ({
    label: t.string(),
    url: t.string(),
  }),
});

ISite.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),
    logo: t.expose('logoId', { type: Image }),
    description: t.exposeString('description', { nullable: true }),
    links: t.field({ type: [SiteLinkObject], resolve: (self) => self.links }),
    allowIndexing: t.exposeBoolean('allowIndexing'),
    allowDiscovery: t.exposeBoolean('allowDiscovery'),

    dateDisplay: t.expose('dateDisplay', { type: SiteDateDisplay }),

    url: t.string({ resolve: (self) => siteUrl(env.USERSITE_URL, self.slug) }),
  }),
});

Site.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  interfaces: [ISite],
  fields: (t) => ({
    view: t.expose('id', { type: SiteView }),
    user: t.expose('userId', { type: User }),

    tagSuggestions: t.field({
      type: TagSuggestions,
      args: {
        query: t.arg.string(),
        exclude: t.arg.stringList({ required: false }),
      },
      resolve: async (self, args, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });
        return await getTagSuggestions({ siteId: self.id, query: args.query, exclude: args.exclude ?? [] });
      },
    }),

    entities: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });

        const loader = ctx.loader({
          name: 'Site.entities',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Entities)
              .where(and(inArray(Entities.siteId, ids), eq(Entities.state, EntityState.ACTIVE), isNull(Entities.parentId)))
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
      resolve: async (self, args, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });

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
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });

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

    recentDocuments: t.field({
      type: RecentDocumentsResult,
      args: {
        sort: t.arg({ type: RecentDocumentSort }),
        limit: t.arg.int({ defaultValue: RECENT_DOCUMENT_DEFAULT_LIMIT }),
      },
      resolve: async (self, args, ctx) => {
        if (ctx.session?.userId !== self.userId) {
          await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });
        }

        const limit = clampRecentDocumentLimit(args.limit);
        const loader = ctx.loader({
          name: `Site.recentDocuments:${self.userId}:${args.sort}:${limit}`,
          many: true,
          load: async (siteIds) =>
            await buildRecentDocumentsBatchQuery(db, {
              userId: self.userId,
              siteIds,
              sort: args.sort,
              limit,
            }),
          key: ({ siteId }) => siteId,
        });

        return toRecentDocumentsPage(await loader.load(self.id), limit);
      },
    }),

    pinnedEntities: t.field({
      type: [Entity],
      resolve: async (self, _, ctx) => {
        if (ctx.session?.userId !== self.userId) {
          await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });
        }

        const loader = ctx.loader({
          name: 'Site.pinnedEntities',
          many: true,
          load: async (siteIds) => await buildPinnedEntitiesBatchQuery(db, { siteIds }),
          key: ({ siteId }) => siteId,
        });

        return await loader.load(self.id);
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
      resolve: async (self, _, ctx) => {
        await assertSitePermission({ userId: ctx.session?.userId, siteId: self.id });

        const parentEntities = alias(Entities, 'parent_entities');
        return await db
          .select(getTableColumns(Entities))
          .from(Entities)
          .leftJoin(parentEntities, eq(Entities.parentId, parentEntities.id))
          .where(
            and(
              eq(Entities.siteId, self.id),
              eq(Entities.state, EntityState.DELETED),
              ne(Entities.type, EntityType.DIVIDER),
              gt(Entities.deletedAt, dayjs().subtract(30, 'days')),
              or(isNull(parentEntities.id), eq(parentEntities.state, EntityState.ACTIVE)),
            ),
          )
          .orderBy(desc(Entities.deletedAt));
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
      description: t.input.string({ required: false }),
      links: t.input.field({ type: [SiteLinkInput], required: false }),
      allowIndexing: t.input.boolean({ required: false }),
      allowDiscovery: t.input.boolean({ required: false }),
      dateDisplay: t.input.field({ type: SiteDateDisplay, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: input.siteId,
      });

      await assertActiveSubscription({ userId: ctx.session.userId });

      const updateData: {
        name?: string;
        logoId?: string;
        description?: string | null;
        links?: SiteLink[];
        allowIndexing?: boolean;
        allowDiscovery?: boolean;
        dateDisplay?: SiteDateDisplay;
      } = {};
      if (input.name !== undefined && input.name !== null) {
        updateData.name = input.name;
      }
      if (input.logoId !== undefined && input.logoId !== null) {
        updateData.logoId = input.logoId;
      }
      if (input.description !== undefined) {
        const description = input.description?.trim() ?? '';
        updateData.description = description.length > 0 ? description : null;
      }
      if (input.links !== undefined && input.links !== null) {
        updateData.links = normalizeSiteLinks(input.links);
      }
      if (input.allowIndexing !== undefined && input.allowIndexing !== null) {
        updateData.allowIndexing = input.allowIndexing;
      }
      if (input.allowDiscovery !== undefined && input.allowDiscovery !== null) {
        updateData.allowDiscovery = input.allowDiscovery;
      }
      if (input.dateDisplay !== undefined && input.dateDisplay !== null) {
        updateData.dateDisplay = input.dateDisplay;
      }

      if (Object.keys(updateData).length === 0) {
        return await db.select().from(Sites).where(eq(Sites.id, input.siteId)).then(firstOrThrow);
      }

      const site = await db.update(Sites).set(updateData).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
      pubsub.publish('site:update', site.id, { scope: 'site' });
      await enqueueDiscoverySiteSync([site.id]);
      return site;
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

      const site = await db.update(Sites).set({ slug: input.slug }).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
      pubsub.publish('site:update', site.id, { scope: 'site' });
      return site;
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

      let unpublishedPublicationIds: string[] = [];

      const site = await db.transaction(async (tx) => {
        const activeSites = await tx
          .select({ id: Sites.id })
          .from(Sites)
          .where(and(eq(Sites.userId, ctx.session.userId), eq(Sites.state, SiteState.ACTIVE)))
          .for('update');

        if (activeSites.length <= 1) {
          throw new TypieError({ code: 'cannot_delete_last_site' });
        }

        unpublishedPublicationIds = await unpublishBySiteIdsCore(tx, { siteIds: [input.siteId], now: dayjs() });

        return await tx.update(Sites).set({ state: SiteState.DELETED }).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
      });

      await enqueueDiscoveryPublicationSync(unpublishedPublicationIds);
      await enqueueDiscoverySiteSync([site.id]);

      return site;
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  siteRecentDocumentsUpdateStream: t.withAuth({ session: true }).field({
    type: RecentDocumentSort,
    args: { siteId: t.arg.id({ validate: validateDbId(TableCode.SITES) }) },
    subscribe: async (_, args, ctx) => {
      await assertSitePermission({ userId: ctx.session.userId, siteId: args.siteId });

      const repeater = pubsub.subscribe('site:recent-documents:update', args.siteId);
      ctx.c.req.raw.signal.addEventListener('abort', () => repeater.return());
      return repeater;
    },
    resolve: (sort) => sort,
  }),

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
