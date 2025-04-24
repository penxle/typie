import { eq } from 'drizzle-orm';
import { db, firstOrThrow, Sites } from '@/db';
import { TypieError } from '@/errors';
import { Site } from '@/graphql/objects';
import type { Context } from '@/context';

type AssertSitePermissionParams = {
  userId: string;
  siteId: string;
  ctx?: Context;
};
export const assertSitePermission = async ({ userId, siteId, ctx }: AssertSitePermissionParams) => {
  const site = ctx
    ? await Site.getDataloader(ctx).load(siteId)
    : await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, siteId)).then(firstOrThrow);

  if (site.userId !== userId) {
    throw new TypieError({ code: 'permission_denied' });
  }
};
