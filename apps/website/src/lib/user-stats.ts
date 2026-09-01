import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

type CharacterCountChange = { date: string; additions: number };
type GoalHistoryEntry = CharacterCountChange & { achieved: boolean };

const isSameDay = (date: string, day: Dayjs): boolean => dayjs(date).kst().isSame(day, 'day');

const withoutToday = <T extends { date: string }>(history: readonly T[], today: Dayjs): T[] =>
  history.filter(({ date }) => !isSameDay(date, today));

export const streaks = (history: readonly { date: string; achieved: boolean }[], today: Dayjs): { current: number; best: number } => {
  const days = history.map((h) => ({ day: dayjs(h.date).kst().startOf('day'), achieved: h.achieved }));

  let best = 0;
  let run = 0;
  let prevAchievedDay: Dayjs | null = null;
  for (const { day, achieved } of days) {
    if (!achieved) {
      run = 0;
      prevAchievedDay = null;
      continue;
    }

    run = prevAchievedDay && day.diff(prevAchievedDay, 'day') === 1 ? run + 1 : 1;
    best = Math.max(best, run);
    prevAchievedDay = day;
  }

  const achievedByDay = new Map(days.map((d) => [d.day.valueOf(), d.achieved]));
  const startOfToday = today.startOf('day');
  let cursor = achievedByDay.get(startOfToday.valueOf()) ? startOfToday : startOfToday.subtract(1, 'day');
  let current = 0;
  while (achievedByDay.get(cursor.valueOf())) {
    current += 1;
    cursor = cursor.subtract(1, 'day');
  }

  return { current, best };
};

export const mergeTodayCharacterCountChanges = <T extends CharacterCountChange>(
  history: readonly T[],
  todayChange: T,
  today: Dayjs,
): T[] => {
  const startOfToday = today.startOf('day');
  const pastChanges = withoutToday(history, startOfToday);

  return isSameDay(todayChange.date, startOfToday) ? [...pastChanges, todayChange] : pastChanges;
};

export const mergeTodayGoalHistory = (
  history: readonly GoalHistoryEntry[],
  target: number,
  todayChange: CharacterCountChange,
  today: Dayjs,
): GoalHistoryEntry[] => {
  const startOfToday = today.startOf('day');
  const additions = isSameDay(todayChange.date, startOfToday) ? todayChange.additions : 0;

  return [...withoutToday(history, startOfToday), { date: startOfToday.toISOString(), additions, achieved: additions >= target }];
};

export const dailyGoalStatus = (
  history: readonly GoalHistoryEntry[],
  target: number,
  todayChange: CharacterCountChange,
  today: Dayjs,
): { additions: number; achieved: boolean; streak: number; bestStreak: number } => {
  const additions = isSameDay(todayChange.date, today) ? todayChange.additions : 0;
  const achieved = additions >= target;
  const liveHistory = mergeTodayGoalHistory(history, target, todayChange, today);
  const liveStreaks = streaks(liveHistory, today);

  return {
    additions,
    achieved,
    streak: liveStreaks.current,
    bestStreak: liveStreaks.best,
  };
};

export const writingStreaks = (history: readonly CharacterCountChange[], today: Dayjs): { current: number; best: number } =>
  streaks(
    history.map(({ date, additions }) => ({ date, achieved: additions > 0 })),
    today,
  );
