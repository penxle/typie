import dayjs from 'dayjs';

export type SessionGroup<T> = { key: string; label: string; sessions: T[] };

export const sessionLabel = (session: { title?: string | null }): string => session.title ?? '제목 없는 대화';

// 최근성 그룹(오너 결정: 기준은 updatedAt = 마지막 대화 시각). 최근일수록 상대 라벨, 오래될수록 절대 라벨로
// 전환한다 — 사람이 대화를 떠올리는 방식과 같다("어제 그거", "지난주쯤", "6월에").
export const groupSessionsByRecency = <T extends { updatedAt: string }>(sessions: readonly T[], now = dayjs()): SessionGroup<T>[] => {
  const today = now.startOf('day');
  const yesterday = today.subtract(1, 'day');
  const weekAgo = today.subtract(7, 'day');
  const monthAgo = today.subtract(30, 'day');

  const bucketOf = (at: dayjs.Dayjs): { key: string; label: string } => {
    if (!at.isBefore(today)) return { key: 'today', label: '오늘' };
    if (!at.isBefore(yesterday)) return { key: 'yesterday', label: '어제' };
    if (!at.isBefore(weekAgo)) return { key: 'week', label: '지난 7일' };
    if (!at.isBefore(monthAgo)) return { key: 'month', label: '지난 30일' };
    return { key: at.format('YYYY-MM'), label: at.year() === today.year() ? at.format('M월') : at.format('YYYY년 M월') };
  };

  const sorted = sessions.toSorted((a, b) => dayjs(b.updatedAt).valueOf() - dayjs(a.updatedAt).valueOf());
  const groups: SessionGroup<T>[] = [];
  for (const session of sorted) {
    const bucket = bucketOf(dayjs(session.updatedAt));
    const last = groups.at(-1);
    if (last && last.key === bucket.key) {
      last.sessions.push(session);
    } else {
      groups.push({ ...bucket, sessions: [session] });
    }
  }
  return groups;
};

export const matchesSessionQuery = (title: string | null | undefined, query: string): boolean => {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return true;
  return (title ?? '').toLowerCase().includes(q);
};
