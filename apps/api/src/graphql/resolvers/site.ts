import { and, asc, eq, inArray, isNull } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, Sites, TableCode, validateDbId } from '@/db';
import { EntityState } from '@/enums';
import { env } from '@/env';
import { pubsub } from '@/pubsub';
import { generateRandomName } from '@/utils/name';
import { assertSitePermission } from '@/utils/permission';
import { builder } from '../builder';
import { Entity, ISite, isTypeOf, Site, SiteView } from '../objects';

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
    resolve: async (payload, args) => {
      return match(payload)
        .with({ scope: 'site' }, () => db.select().from(Sites).where(eq(Sites.id, args.siteId)).then(firstOrThrow))
        .with({ scope: 'entity' }, ({ entityId }) => db.select().from(Entities).where(eq(Entities.id, entityId)).then(firstOrThrow))
        .exhaustive();
    },
  }),
}));
