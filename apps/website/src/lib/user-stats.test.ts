import '@typie/lib/dayjs';

import dayjs from 'dayjs';
import { describe, expect, test } from 'vitest';
import { dailyGoalStatus, mergeTodayCharacterCountChanges, mergeTodayGoalHistory, streaks, writingStreaks } from './user-stats';

describe('streaks', () => {
  const today = dayjs.kst('2026-08-05');
  const d = (s: string) => dayjs.kst(s).toISOString();

  test('오늘 달성 포함 연속', () => {
    const history = [
      { date: d('2026-08-03'), achieved: true },
      { date: d('2026-08-04'), achieved: true },
      { date: d('2026-08-05'), achieved: true },
    ];

    expect(streaks(history, today)).toEqual({ current: 3, best: 3 });
  });

  test('오늘 미달성은 어제까지의 연속으로', () => {
    const history = [
      { date: d('2026-08-03'), achieved: true },
      { date: d('2026-08-04'), achieved: true },
      { date: d('2026-08-05'), achieved: false },
    ];

    expect(streaks(history, today)).toEqual({ current: 2, best: 2 });
  });

  test('미달성 하루가 연속을 끊음', () => {
    const history = [
      { date: d('2026-08-01'), achieved: true },
      { date: d('2026-08-02'), achieved: false },
      { date: d('2026-08-03'), achieved: true },
      { date: d('2026-08-04'), achieved: true },
      { date: d('2026-08-05'), achieved: true },
    ];

    expect(streaks(history, today)).toEqual({ current: 3, best: 3 });
  });

  test('목표 없던 날(행 없음)도 연속을 끊음', () => {
    const history = [
      { date: d('2026-08-01'), achieved: true },
      { date: d('2026-08-02'), achieved: true },
      { date: d('2026-08-04'), achieved: true },
      { date: d('2026-08-05'), achieved: true },
    ];

    expect(streaks(history, today)).toEqual({ current: 2, best: 2 });
  });

  test('최고 기록은 과거 구간에서', () => {
    const history = [
      { date: d('2026-07-28'), achieved: true },
      { date: d('2026-07-29'), achieved: true },
      { date: d('2026-07-30'), achieved: true },
      { date: d('2026-07-31'), achieved: true },
      { date: d('2026-08-01'), achieved: false },
      { date: d('2026-08-04'), achieved: true },
      { date: d('2026-08-05'), achieved: true },
    ];

    expect(streaks(history, today)).toEqual({ current: 2, best: 4 });
  });

  test('빈 이력은 0', () => {
    expect(streaks([], today)).toEqual({ current: 0, best: 0 });
  });
});

describe('dailyGoalStatus', () => {
  const today = dayjs.kst('2026-08-05');
  const d = (s: string) => dayjs.kst(s).toISOString();

  test('서버의 오늘 통계로 오래된 목표 진행률과 연속일을 덮어쓴다', () => {
    const history = [
      { date: d('2026-08-03'), additions: 1000, achieved: true },
      { date: d('2026-08-04'), additions: 1000, achieved: true },
      { date: d('2026-08-05'), additions: 0, achieved: false },
    ];

    expect(dailyGoalStatus(history, 1000, { date: d('2026-08-05'), additions: 1200 }, today)).toEqual({
      additions: 1200,
      achieved: true,
      streak: 3,
      bestStreak: 3,
    });
  });
});

describe('mergeTodayCharacterCountChanges', () => {
  const today = dayjs.kst('2026-08-05');
  const d = (s: string) => dayjs.kst(s).toISOString();

  test('오늘의 오래된 글자 수 행을 서버의 최신 행으로 교체한다', () => {
    const history = [
      { date: d('2026-08-03'), additions: 100 },
      { date: d('2026-08-04'), additions: 100 },
      { date: d('2026-08-05'), additions: 0 },
    ];
    const todayChange = { date: d('2026-08-05'), additions: 50 };

    expect(mergeTodayCharacterCountChanges(history, todayChange, today)).toEqual([
      { date: d('2026-08-03'), additions: 100 },
      { date: d('2026-08-04'), additions: 100 },
      todayChange,
    ]);
  });
});

describe('mergeTodayGoalHistory', () => {
  const today = dayjs.kst('2026-08-05');
  const d = (s: string) => dayjs.kst(s).toISOString();

  test('서버의 오늘 글자 수가 감소하면 오래된 달성 기록을 미달성으로 교체한다', () => {
    const history = [
      { date: d('2026-08-04'), additions: 1200, achieved: true },
      { date: d('2026-08-05'), additions: 1200, achieved: true },
    ];

    expect(mergeTodayGoalHistory(history, 1000, { date: d('2026-08-05'), additions: 0 }, today)).toEqual([
      { date: d('2026-08-04'), additions: 1200, achieved: true },
      { date: today.startOf('day').toISOString(), additions: 0, achieved: false },
    ]);
  });
});

describe('writingStreaks', () => {
  const today = dayjs.kst('2026-08-05');
  const d = (s: string) => dayjs.kst(s).toISOString();

  test('같은 글자 수 이력에서 현재와 최장 연속 기록을 함께 계산한다', () => {
    const history = [
      { date: d('2026-08-01'), additions: 100 },
      { date: d('2026-08-02'), additions: 100 },
      { date: d('2026-08-04'), additions: 100 },
      { date: d('2026-08-05'), additions: 50 },
    ];

    expect(writingStreaks(history, today)).toEqual({ current: 2, best: 2 });
  });
});
