import { and, eq, sum } from 'drizzle-orm';
import { db, Entities, firstOrThrow, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';

type GetUserUsageParams = {
  userId: string;
};
export const getUserUsage = async ({ userId }: GetUserUsageParams) => {
  const row = await db
    .select({
      totalCharacterCount: sum(PostContents.characterCount).mapWith(Number),
      totalBlobSize: sum(PostContents.blobSize).mapWith(Number),
    })
    .from(PostContents)
    .innerJoin(Posts, eq(PostContents.postId, Posts.id))
    .innerJoin(Entities, eq(Posts.entityId, Entities.id))
    .where(and(eq(Entities.userId, userId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  return {
    totalCharacterCount: row.totalCharacterCount || 0,
    totalBlobSize: row.totalBlobSize || 0,
  };
};
