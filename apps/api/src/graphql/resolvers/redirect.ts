import { and, eq } from 'drizzle-orm';
import { db, first, Redirects, TableCode } from '#/db/index.ts';
import { RedirectType } from '#/enums.ts';
import { builder } from '../builder.ts';
import { isTypeOf, Redirect } from '../objects.ts';

/**
 * * Types
 */

Redirect.implement({
  isTypeOf: isTypeOf(TableCode.REDIRECTS),
  fields: (t) => ({
    id: t.exposeID('id'),
    type: t.expose('type', { type: RedirectType }),
    from: t.exposeString('from'),
    to: t.exposeString('to'),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  redirect: t.field({
    type: Redirect,
    nullable: true,
    args: {
      type: t.arg({ type: RedirectType }),
      from: t.arg.string(),
    },
    resolve: async (_, args) => {
      return await db
        .select()
        .from(Redirects)
        .where(and(eq(Redirects.type, args.type), eq(Redirects.from, args.from)))
        .then(first);
    },
  }),
}));
