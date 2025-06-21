import { eq } from 'drizzle-orm';
import { db, firstOrThrow, Sites, Users } from '@/db';
import { UserRole } from '@/enums';
import { TypieError } from '@/errors';

type AssertAdminPermissionParams = {
  userId: string | undefined;
};
export const assertAdminPermission = async ({ userId }: AssertAdminPermissionParams) => {
  if (!userId) {
    throw new TypieError({ code: 'permission_denied' });
  }

  const user = await db.select({ role: Users.role }).from(Users).where(eq(Users.id, userId)).then(firstOrThrow);

  if (user.role !== UserRole.ADMIN) {
    throw new TypieError({ code: 'permission_denied' });
  }
};

type AssertSitePermissionParams = {
  userId: string | undefined;
  siteId: string;
};
export const assertSitePermission = async ({ userId, siteId }: AssertSitePermissionParams) => {
  if (!userId) {
    throw new TypieError({ code: 'permission_denied' });
  }

  const user = await db.select({ role: Users.role }).from(Users).where(eq(Users.id, userId)).then(firstOrThrow);

  if (user.role === UserRole.ADMIN) {
    return;
  }

  const site = await db.select({ userId: Sites.userId }).from(Sites).where(eq(Sites.id, siteId)).then(firstOrThrow);

  if (site.userId !== userId) {
    throw new TypieError({ code: 'permission_denied' });
  }
};
