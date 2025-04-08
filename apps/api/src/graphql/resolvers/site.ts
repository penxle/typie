import { and, asc, eq, isNull } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, Entities, firstOrThrow, Sites, TableCode } from '@/db';
import { EntityState } from '@/enums';
import { env } from '@/env';
import { pubsub } from '@/pubsub';
import { builder } from '../builder';
import { Entity, isTypeOf, Site } from '../objects';

/**
 * * Types
 */

Site.implement({
  isTypeOf: isTypeOf(TableCode.SITES),
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),

    url: t.string({
      resolve: (self) => env.USERSITE_URL.replace('*', self.slug),
    }),

    entities: t.field({
      type: [Entity],
      resolve: async (site) => {
        return await db
          .select()
          .from(Entities)
          .where(and(eq(Entities.siteId, site.id), eq(Entities.state, EntityState.ACTIVE), isNull(Entities.parentId)))
          .orderBy(asc(Entities.order));
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
    args: { siteId: t.arg.id() },
    resolve: async (_, args) => {
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
    args: { siteId: t.arg.id() },
    subscribe: async (_, args, ctx) => {
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
