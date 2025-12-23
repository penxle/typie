import { and, eq, sum } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, firstOrThrow, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';

type GetUserUsageParams = {
  userId: string;
};
export const getUserUsage = async ({ userId }: GetUserUsageParams) => {
  const postUsage = await db
    .select({
      totalCharacterCount: sum(PostContents.characterCount).mapWith(Number),
      totalBlobSize: sum(PostContents.blobSize).mapWith(Number),
    })
    .from(PostContents)
    .innerJoin(Posts, eq(PostContents.postId, Posts.id))
    .innerJoin(Entities, eq(Posts.entityId, Entities.id))
    .where(and(eq(Entities.userId, userId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  const documentUsage = await db
    .select({
      totalCharacterCount: sum(DocumentContents.characterCount).mapWith(Number),
      totalBlobSize: sum(DocumentContents.blobSize).mapWith(Number),
    })
    .from(DocumentContents)
    .innerJoin(Documents, eq(DocumentContents.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(and(eq(Entities.userId, userId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  return {
    totalCharacterCount: (postUsage.totalCharacterCount || 0) + (documentUsage.totalCharacterCount || 0),
    totalBlobSize: (postUsage.totalBlobSize || 0) + (documentUsage.totalBlobSize || 0),
  };
};
