import { eq } from 'drizzle-orm';
import { db, firstOrThrow, Sites } from '@/db';
import { TypieError } from '@/errors';

type AssertSitePermissionParams = {
  userId: string;
  siteId: string;
};
export const assertSitePermission = async ({ userId, siteId }: AssertSitePermissionParams) => {
  const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, siteId)).then(firstOrThrow);

  if (site.userId !== userId) {
    throw new TypieError({ code: 'permission_denied' });
  }
};
