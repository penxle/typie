import { EntityState } from '@typie/lib/enums';
import { and, eq, sql, sum } from 'drizzle-orm';
import * as uuid from 'uuid';
import { db, DocumentContents, Documents, DocumentStates, Entities, firstOrThrow } from '#/db/index.ts';

// 유저 id 로부터 결정적으로 파생하는 외부 계정 식별자. IAP 구매 시 appAccountToken / obfuscatedAccountId 로 쓰이며,
// 등록/복구 시 스토어가 돌려준 계정 식별자와 대조해 다른 유저의 구매가 현재 세션에 바인딩되는 것을 막는다.
// 네임스페이스 값 자체는 의미가 없고 그냥 임의로 고른 UUID다 — 안정적으로 고정돼 있기만 하면 된다(바꾸면 기존 식별자가 전부 달라짐).
const USER_UUID_NAMESPACE = '1d394eb5-c61c-4c49-944e-05c9f9435adf';

export const getUserUuid = (userId: string): string => uuid.v5(userId, USER_UUID_NAMESPACE);

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
