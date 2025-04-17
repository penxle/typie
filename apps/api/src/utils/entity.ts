import { eq } from 'drizzle-orm';
import { db, Entities, firstOrThrow } from '@/db';
import { Entity } from '@/graphql/objects';
import { assertSitePermission } from './permission';
import type { Context } from '@/context';

type CheckEntityPermissionParams = {
  entityId: string;
  userId: string | undefined;
  ctx?: Context;
};
export const checkEntityPermission = async ({ entityId, userId, ctx }: CheckEntityPermissionParams) => {
  if (!userId) {
    return false;
  }

  const entity = ctx
    ? await Entity.getDataloader(ctx).load(entityId)
    : await db.select().from(Entities).where(eq(Entities.id, entityId)).then(firstOrThrow);

  if (entity.userId === userId) {
    return true;
  }

  return assertSitePermission({
    siteId: entity.siteId,
    userId,
    ctx,
  })
    .then(() => true)
    .catch(() => false);
};
