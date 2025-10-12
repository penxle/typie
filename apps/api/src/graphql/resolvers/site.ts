import dayjs from 'dayjs';
import { and, asc, desc, eq, getTableColumns, gt, inArray, isNull, or, sql, sum } from 'drizzle-orm';
import { alias } from 'drizzle-orm/pg-core';
import { match } from 'ts-pattern';
import { clearLoaders } from '@/context';
import { db, Entities, first, firstOrThrow, FontFamilies, Fonts, PostContents, Posts, Sites, TableCode, validateDbId } from '@/db';
import { EntityState, EntityType, FontState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import { pubsub } from '@/pubsub';
import { generateRandomName } from '@/utils/name';
import { assertSitePermission } from '@/utils/permission';
import { siteSchema } from '@/validation';
import { builder } from '../builder';
import { Entity, Font, ISite, isTypeOf, Post, Site, SiteView, User } from '../objects';

/**
 * * Types
 */

ISite.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    slug: t.exposeString('slug'),
    name: t.exposeString('name'),

    url: t.string({ resolve: (self) => env.USERSITE_URL.replace('*', self.slug) }),

    fonts: t.field({
      type: [Font],
      resolve: async (self) => {
        const fonts = await db
          .select(getTableColumns(Fonts))
          .from(Fonts)
          .innerJoin(FontFamilies, eq(Fonts.familyId, FontFamilies.id))
          .where(and(eq(FontFamilies.userId, self.userId), eq(Fonts.state, FontState.ACTIVE)));

        return fonts.toSorted((a, b) => a.name.localeCompare(b.name));
      },
    }),
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

    templates: t.field({
      type: [Post],
      resolve: async () => {
        return [];
      },
    }),

    usage: t.field({
      deprecationReason: 'Use User.usage instead',
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
}));
