import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

export type UserGoalEntry = {
  targetCharacterCount: number | null;
  effectiveAt: Dayjs;
};

export const getEffectiveTarget = (entries: UserGoalEntry[], date: Dayjs): number | null => {
  let latest: Dayjs | null = null;
  let target: number | null = null;

  for (const entry of entries) {
    if (entry.effectiveAt.isAfter(date)) {
      continue;
    }

    if (!latest || entry.effectiveAt.isAfter(latest)) {
      latest = entry.effectiveAt;
      target = entry.targetCharacterCount;
    }
  }

  return target;
};

export type DocumentDailyCount = {
  documentId: string;
  date: string;
  characterCount: number;
};

export const buildDailyHistory = (rows: DocumentDailyCount[], until: string): { date: string; characterCount: number }[] => {
  if (rows.length === 0) {
    return [];
  }

  const byDate = new Map<string, Map<string, number>>();
  let first = rows[0].date;

  for (const row of rows) {
    if (row.date < first) {
      first = row.date;
    }

    const perDoc = byDate.get(row.date) ?? new Map<string, number>();
    perDoc.set(row.documentId, row.characterCount);
    byDate.set(row.date, perDoc);
  }

  const result: { date: string; characterCount: number }[] = [];
  const lastByDoc = new Map<string, number>();

  let cursor = dayjs.kst(first).startOf('day');
  const end = dayjs.kst(until).startOf('day');

  while (!cursor.isAfter(end)) {
    const date = cursor.format('YYYY-MM-DD');
    const perDoc = byDate.get(date);

    if (perDoc) {
      for (const [documentId, count] of perDoc) {
        lastByDoc.set(documentId, count);
      }
    }

    let total = 0;
    for (const count of lastByDoc.values()) {
      total += count;
    }

    result.push({ date, characterCount: total });
    cursor = cursor.add(1, 'day');
  }

  return result;
};
