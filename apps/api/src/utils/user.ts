import { EntityState } from '@typie/lib/enums';
import { and, eq, sql, sum } from 'drizzle-orm';
import { db, DocumentContents, Documents, DocumentStates, Entities, firstOrThrow } from '#/db/index.ts';

type GetUserUsageParams = {
  userId: string;
};
export const getUserUsage = async ({ userId }: GetUserUsageParams) => {
  const documentUsage = await db
    .select({
      totalCharacterCount: sum(sql`COALESCE(${DocumentStates.characterCount}, ${DocumentContents.characterCount})`).mapWith(Number),
      totalBlobSize: sum(sql`COALESCE(${DocumentStates.blobSize}, ${DocumentContents.blobSize})`).mapWith(Number),
    })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .innerJoin(DocumentContents, eq(DocumentContents.documentId, Documents.id))
    .leftJoin(DocumentStates, eq(DocumentStates.documentId, Documents.id))
    .where(and(eq(Entities.userId, userId), eq(Entities.state, EntityState.ACTIVE)))
    .then(firstOrThrow);

  return {
    totalCharacterCount: documentUsage.totalCharacterCount || 0,
    totalBlobSize: documentUsage.totalBlobSize || 0,
  };
};
