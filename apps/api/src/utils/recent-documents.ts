import { EntityState, EntityType } from '@typie/lib/enums';
import { and, asc, eq, getTableColumns, gt, inArray, isNotNull, isNull, lte, or, sql } from 'drizzle-orm';
import { Documents, Entities } from '#/db/schemas/tables.ts';
import type { PgColumn } from 'drizzle-orm/pg-core';
import type { Database } from '#/db/index.ts';

export const RECENT_DOCUMENT_DEFAULT_LIMIT = 5;
export const RECENT_DOCUMENT_LIMIT = 50;
export const RECENT_DOCUMENT_SORTS = ['VIEWED_AT', 'UPDATED_AT'] as const;

export type RecentDocumentSort = (typeof RECENT_DOCUMENT_SORTS)[number];

export const clampRecentDocumentLimit = (limit: number) => Math.min(Math.max(limit, 1), RECENT_DOCUMENT_LIMIT);

export const notDismissedFromRecent = (activityColumn: PgColumn) =>
  or(isNull(Entities.recentDismissedAt), gt(activityColumn, Entities.recentDismissedAt));

export const buildRecentDocumentsBatchQuery = (
  executor: Database,
  input: { userId: string; siteIds: string[]; sort: RecentDocumentSort; limit: number },
) => {
  const limit = clampRecentDocumentLimit(input.limit);
  const queryLimit = limit < RECENT_DOCUMENT_LIMIT ? limit + 1 : limit;
  const sortColumn = input.sort === 'VIEWED_AT' ? Entities.viewedAt : Documents.updatedAt;
  const recentRank = sql<number>`row_number() over (partition by ${Entities.siteId} order by ${sortColumn} desc, ${Documents.id} desc)`.as(
    'recent_rank',
  );
  const rankedDocuments = executor
    .select({ ...getTableColumns(Documents), siteId: Entities.siteId, recentRank })
    .from(Documents)
    .innerJoin(Entities, eq(Documents.entityId, Entities.id))
    .where(
      and(
        eq(Entities.userId, input.userId),
        inArray(Entities.siteId, input.siteIds),
        eq(Entities.state, EntityState.ACTIVE),
        eq(Entities.type, EntityType.DOCUMENT),
        input.sort === 'VIEWED_AT' ? isNotNull(Entities.viewedAt) : undefined,
        notDismissedFromRecent(sortColumn),
      ),
    )
    .as('ranked_recent_documents');

  return executor
    .select()
    .from(rankedDocuments)
    .where(lte(rankedDocuments.recentRank, queryLimit))
    .orderBy(asc(rankedDocuments.siteId), asc(rankedDocuments.recentRank));
};

export const toRecentDocumentsPage = <T>(documents: T[], requestedLimit: number) => {
  const limit = clampRecentDocumentLimit(requestedLimit);
  return {
    documents: documents.slice(0, limit),
    hasMore: limit < RECENT_DOCUMENT_LIMIT && documents.length > limit,
  };
};
