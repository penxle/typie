import { and, asc, eq, isNull } from 'drizzle-orm';
import { db, Entities, TableCode } from '@/db';
import { EntityState } from '@/enums';
import { env } from '@/env';
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
