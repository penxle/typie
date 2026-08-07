import { and, eq, gte, lt, sql } from 'drizzle-orm';
import { db, DocumentHeadContributors, DocumentHeads } from '#/db/index.ts';
import type { Dayjs } from 'dayjs';

export const getExcludedDeltasByDate = async (params: {
  userId: string;
  from: Dayjs;
  to: Dayjs;
  documentId?: string;
}): Promise<Map<string, { additions: number; deletions: number }>> => {
  const date = sql<string>`TO_CHAR(${DocumentHeads.updatedAt} AT TIME ZONE 'Asia/Seoul', 'YYYY-MM-DD')`;

  const rows = await db
    .select({
      date,
      additions: sql`COALESCE(SUM(${DocumentHeadContributors.additions}), 0)`.mapWith(Number),
      deletions: sql`COALESCE(SUM(${DocumentHeadContributors.deletions}), 0)`.mapWith(Number),
    })
    .from(DocumentHeads)
    .innerJoin(DocumentHeadContributors, eq(DocumentHeadContributors.headId, DocumentHeads.id))
    .where(
      and(
        eq(DocumentHeadContributors.userId, params.userId),
        eq(DocumentHeadContributors.excluded, true),
        gte(DocumentHeads.updatedAt, params.from),
        lt(DocumentHeads.updatedAt, params.to),
        params.documentId ? eq(DocumentHeads.documentId, params.documentId) : undefined,
      ),
    )
    .groupBy(date);

  return new Map(rows.map((row) => [row.date, { additions: row.additions, deletions: row.deletions }]));
};
