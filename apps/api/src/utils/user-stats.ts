import dayjs from 'dayjs';
import { and, desc, eq, gte, lt, lte, sql, sum } from 'drizzle-orm';
import { db, DocumentCharacterCountChanges, first, UserGoals } from '#/db/index.ts';
import { getExcludedDeltasByDate } from '#/utils/excluded-stats.ts';
import { getEffectiveTarget } from '#/utils/goal.ts';
import type { Dayjs } from 'dayjs';

export const currentUserGoal = async (userId: string): Promise<{ id: string; targetCharacterCount: number } | null> => {
  const row = await db
    .select({ id: UserGoals.id, targetCharacterCount: UserGoals.targetCharacterCount })
    .from(UserGoals)
    .where(and(eq(UserGoals.userId, userId), lte(UserGoals.effectiveAt, dayjs.kst().startOf('day'))))
    .orderBy(desc(UserGoals.effectiveAt))
    .limit(1)
    .then(first);

  return row && row.targetCharacterCount !== null ? { id: row.id, targetCharacterCount: row.targetCharacterCount } : null;
};

export const dailyGoalHistory = async (
  userId: string,
): Promise<{ date: Dayjs; targetCharacterCount: number; additions: number; achieved: boolean }[]> => {
  const startOfToday = dayjs.kst().startOf('day');
  const startOfTomorrow = startOfToday.add(1, 'day');
  const from = startOfTomorrow.subtract(365, 'days');

  const goalRows = await db
    .select({ targetCharacterCount: UserGoals.targetCharacterCount, effectiveAt: UserGoals.effectiveAt })
    .from(UserGoals)
    .where(and(eq(UserGoals.userId, userId), lt(UserGoals.effectiveAt, startOfTomorrow)));

  const documentDate = sql<string>`DATE(${DocumentCharacterCountChanges.bucket} AT TIME ZONE 'Asia/Seoul')`.mapWith(dayjs.kst);
  const additionRows = await db
    .select({ date: documentDate, additions: sum(DocumentCharacterCountChanges.additions).mapWith(Number) })
    .from(DocumentCharacterCountChanges)
    .where(
      and(
        eq(DocumentCharacterCountChanges.userId, userId),
        gte(DocumentCharacterCountChanges.bucket, from),
        lt(DocumentCharacterCountChanges.bucket, startOfTomorrow),
      ),
    )
    .groupBy(documentDate);

  const additionsByDate = new Map(additionRows.map((row) => [row.date.format('YYYY-MM-DD'), row.additions]));

  const excludedByDate = await getExcludedDeltasByDate({ userId, from, to: startOfTomorrow });

  const result = [];
  let cursor = from;
  while (!cursor.isAfter(startOfToday)) {
    const target = getEffectiveTarget(goalRows, cursor);
    if (target !== null && target > 0) {
      const key = cursor.format('YYYY-MM-DD');
      const additions = (additionsByDate.get(key) ?? 0) - (excludedByDate.get(key)?.additions ?? 0);
      result.push({ date: cursor, targetCharacterCount: target, additions, achieved: additions >= target });
    }
    cursor = cursor.add(1, 'day');
  }

  return result;
};

const characterChangesInRange = async (
  userId: string,
  from: Dayjs,
  to: Dayjs,
): Promise<{ date: Dayjs; additions: number; deletions: number }[]> => {
  const documentDate = sql<string>`DATE(${DocumentCharacterCountChanges.bucket} AT TIME ZONE 'Asia/Seoul')`.mapWith(dayjs.kst);

  const excludedByDate = await getExcludedDeltasByDate({ userId, from, to });

  const rows = await db
    .select({
      date: documentDate,
      additions: sum(DocumentCharacterCountChanges.additions).mapWith(Number),
      deletions: sum(DocumentCharacterCountChanges.deletions).mapWith(Number),
    })
    .from(DocumentCharacterCountChanges)
    .where(
      and(
        eq(DocumentCharacterCountChanges.userId, userId),
        gte(DocumentCharacterCountChanges.bucket, from),
        lt(DocumentCharacterCountChanges.bucket, to),
      ),
    )
    .groupBy(documentDate)
    .orderBy(documentDate);

  return rows.map((row) => {
    const excluded = excludedByDate.get(row.date.format('YYYY-MM-DD'));

    return {
      ...row,
      additions: row.additions - (excluded?.additions ?? 0),
      deletions: row.deletions - (excluded?.deletions ?? 0),
    };
  });
};

export const dailyCharacterChanges = async (userId: string): Promise<{ date: Dayjs; additions: number; deletions: number }[]> => {
  const startOfTomorrow = dayjs.kst().startOf('day').add(1, 'day');
  return await characterChangesInRange(userId, startOfTomorrow.subtract(365, 'days'), startOfTomorrow);
};

export const todayCharacterCountChange = async (userId: string): Promise<{ date: Dayjs; additions: number; deletions: number }> => {
  const startOfToday = dayjs.kst().startOf('day');
  const rows = await characterChangesInRange(userId, startOfToday, startOfToday.add(1, 'day'));
  return rows[0] ?? { date: startOfToday, additions: 0, deletions: 0 };
};
