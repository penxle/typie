import { and, asc, eq, inArray, isNull } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { clearLoaders } from '@/context';
import { db, Entities, first, firstOrThrow, SiteProfiles, Sites, TableCode, validateDbId } from '@/db';
import { EntityState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import { pubsub } from '@/pubsub';
import { generateRandomName } from '@/utils/name';
import { assertSitePermission } from '@/utils/permission';
import { siteSchema } from '@/validation';
import { builder } from '../builder';
import { Entity, ISite, isTypeOf, Site, SiteProfile, SiteView } from '../objects';

/**
 * * Types
 */

ISite.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),

    url: t.string({ resolve: (self) => env.USERSITE_URL.replace('*', self.slug) }),
  }),
});

Site.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  interfaces: [ISite],
  fields: (t) => ({
    view: t.expose('id', { type: SiteView }),

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
              .where(and(inArray(Entities.siteId, ids), eq(Entities.state, EntityState.ACTIVE), isNull(Entities.parentId)))
              .orderBy(asc(Entities.order));
          },
          key: ({ siteId }) => siteId,
        });

        return await loader.load(self.id);
      },
    }),
  }),
});

SiteView.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  interfaces: [ISite],
  fields: (t) => ({
    myMasqueradeName: t.string({
      resolve: async (self, _, ctx) => {
        return generateRandomName(`${self.id}:${ctx.session?.userId ?? ctx.deviceId}`);
      },
    }),

    mySiteProfile: t.field({
      type: SiteProfile,
      nullable: true,
      resolve: async (self, _, ctx) => {
        if (!ctx.session) {
          return null;
        }

        return await db
          .select()
          .from(SiteProfiles)
          .where(and(eq(SiteProfiles.siteId, self.id), eq(SiteProfiles.userId, ctx.session.userId)))
          .then(first);
      },
    }),
  }),
});

SiteProfile.implement({
  isTypeOf: isTypeOf(TableCode.SITE_PROFILES),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
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

      const slugExistSite = await db.select({ id: Sites.id }).from(Sites).where(eq(Sites.slug, input.slug)).then(first);

      if (slugExistSite) {
        throw new TypieError({ code: 'site_slug_already_exists' });
      }

      return await db.update(Sites).set({ slug: input.slug }).where(eq(Sites.id, input.siteId)).returning().then(firstOrThrow);
    },
  }),

  createSiteProfile: t.withAuth({ session: true }).fieldWithInput({
    type: SiteProfile,
    input: { siteId: t.input.id({ validate: validateDbId(TableCode.SITES) }) },
    resolve: async (_, { input }, ctx) => {
      const site = await db.select({ id: Sites.id }).from(Sites).where(eq(Sites.id, input.siteId)).then(firstOrThrow);

      return await db
        .insert(SiteProfiles)
        .values({
          siteId: site.id,
          userId: ctx.session.userId,
          name: generateRandomName(`${site.id}:${ctx.session.userId}`),
        })
        .returning()
        .then(firstOrThrow);
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
}));
