import { EntityState } from '@typie/lib/enums';
import { and, eq, sum } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, firstOrThrow } from '#/db/index.ts';

type GetUserUsageParams = {
  userId: string;
};
export const getUserUsage = async ({ userId }: GetUserUsageParams) => {
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
    totalCharacterCount: documentUsage.totalCharacterCount || 0,
    totalBlobSize: documentUsage.totalBlobSize || 0,
  };
};
