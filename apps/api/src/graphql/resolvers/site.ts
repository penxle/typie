import { and, asc, eq, inArray, isNull, sql, sum } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { clearLoaders } from '@/context';
import { db, Entities, first, firstOrThrow, PostContents, Posts, Sites, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import { pubsub } from '@/pubsub';
import { generateRandomName } from '@/utils/name';
import { assertSitePermission } from '@/utils/permission';
import { siteSchema } from '@/validation';
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

    firstEntity: t.field({
      type: Entity,
      nullable: true,
      args: { type: t.arg({ type: EntityType }) },
      resolve: async (self, args) => {
        const row = await db
          .execute<{ id: string }>(
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
          )
          .then(first);

        return row?.id;
      },
    }),

    usage: t.field({
      type: t.builder.simpleObject('SiteUsage', {
        fields: (t) => ({
          totalCharacterCount: t.int(),
          totalBlobSize: t.int(),
        }),
      }),
      resolve: async (self) => {
        const row = await db
          .select({
            totalCharacterCount: sum(PostContents.characterCount).mapWith(Number),
            totalBlobSize: sum(PostContents.blobSize).mapWith(Number),
          })
          .from(PostContents)
          .innerJoin(Posts, eq(PostContents.postId, Posts.id))
          .innerJoin(Entities, eq(Posts.entityId, Entities.id))
          .where(and(eq(Entities.siteId, self.id), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);

        return {
          totalCharacterCount: row.totalCharacterCount || 0,
          totalBlobSize: row.totalBlobSize || 0,
        };
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
      await assertSitePermission({
        userId: ctx.session.userId,
        siteId: args.siteId,
      });

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
}));
