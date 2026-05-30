import { EntityAvailability, EntityState, UserRole } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { db, Documents, Entities, firstOrThrow, Sites, Users, UserSessions } from '#/db/index.ts';

type AssertAdminPermissionParams = {
  sessionId: string;
};
export const assertAdminPermission = async ({ sessionId }: AssertAdminPermissionParams) => {
  const user = await db
    .select({ role: Users.role })
    .from(UserSessions)
    .innerJoin(Users, eq(UserSessions.userId, Users.id))
    .where(eq(UserSessions.id, sessionId))
    .then(firstOrThrow);

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

type AssertDocumentCommentAccessParams = {
  userId: string | undefined;
  documentId: string;
};
export const assertDocumentCommentAccess = async ({
  userId,
  documentId,
}: AssertDocumentCommentAccessParams): Promise<{ isOwner: boolean }> => {
  if (!userId) {
    throw new TypieError({ code: 'permission_denied' });
  }

  const user = await db.select({ role: Users.role }).from(Users).where(eq(Users.id, userId)).then(firstOrThrow);

  const row = await db
    .select({ availability: Entities.availability, state: Entities.state, siteUserId: Sites.userId })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .innerJoin(Sites, eq(Entities.siteId, Sites.id))
    .where(eq(Documents.id, documentId))
    .then(firstOrThrow);

  // 삭제된 엔티티에는 코멘트 접근 불가 (소유자/admin 포함)
  if (row.state !== EntityState.ACTIVE) {
    throw new TypieError({ code: 'permission_denied' });
  }

  const isOwner = user.role === UserRole.ADMIN || row.siteUserId === userId;
  if (isOwner) {
    return { isOwner: true };
  }

  // 비소유자: PRIVATE은 거부, UNLISTED는 로그인 사용자면 허용
  if (row.availability === EntityAvailability.PRIVATE) {
    throw new TypieError({ code: 'permission_denied' });
  }

  return { isOwner: false };
};
