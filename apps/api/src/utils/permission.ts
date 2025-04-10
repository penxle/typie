import { eq, inArray } from 'drizzle-orm';
import { db, firstOrThrow, Sites } from '@/db';
import { TypieError } from '@/errors';
import type { Context } from '@/context';

type AssertSitePermissionParams = {
  userId: string;
  siteId: string;
  ctx?: Context;
};
export const assertSitePermission = async ({ userId, siteId, ctx }: AssertSitePermissionParams) => {
  const loader = ctx
    ? ctx.loader({
        name: 'Sites(id)',
        load: async (ids: string[]) => {
          return await db.select().from(Sites).where(inArray(Sites.id, ids));
        },
        key: (sites) => sites.id,
      })
    : null;

  const site = loader ? await loader.load(siteId) : await db.select().from(Sites).where(eq(Sites.id, siteId)).then(firstOrThrow);

  if (site.userId !== userId) {
    throw new TypieError({ code: 'permission_denied' });
  }
};
