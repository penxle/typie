import '@typie/lib/dayjs';

import dayjs from 'dayjs';
import { describe, expect, test } from 'vitest';
import {
  dDayLabel,
  dueStatus,
  GOAL_EXCESS_RATIO,
  GOAL_OVER_RATIO,
  goalColorState,
  pickGoalSource,
  requiredToday,
  timeFraction,
} from './goal';

const day = (s: string) => dayjs(s).startOf('day');

describe('goalColorState', () => {
  test('미달은 under, 경계 100%는 achieved', () => {
    expect(goalColorState(0, 1000)).toBe('under');
    expect(goalColorState(999, 1000)).toBe('under');
    expect(goalColorState(1000, 1000)).toBe('achieved');
  });

  test('110% 이하 achieved, 초과~125% over, 그 위 excess', () => {
    expect(goalColorState(1100, 1000)).toBe('achieved');
    expect(goalColorState(1101, 1000)).toBe('over');
    expect(goalColorState(1250, 1000)).toBe('over');
    expect(goalColorState(1251, 1000)).toBe('excess');
  });

  test('상수가 표 1과 일치', () => {
    expect(GOAL_OVER_RATIO).toBe(1.1);
    expect(GOAL_EXCESS_RATIO).toBe(1.25);
  });
});

describe('requiredToday', () => {
  test('잔여일로 균등 분할(오늘 포함, 올림)', () => {
    expect(requiredToday(1000, 10_000, day('2026-08-07'), day('2026-08-05'))).toBe(3000);
  });

  test('마감 당일은 남은 전량', () => {
    expect(requiredToday(1000, 10_000, day('2026-08-05'), day('2026-08-05'))).toBe(9000);
  });

  test('마감 경과도 남은 전량', () => {
    expect(requiredToday(1000, 10_000, day('2026-08-01'), day('2026-08-05'))).toBe(9000);
  });

  test('달성 후에는 0', () => {
    expect(requiredToday(10_000, 10_000, day('2026-08-07'), day('2026-08-05'))).toBe(0);
  });
});

describe('timeFraction', () => {
  test('구간 중간이면 비율(마감일 포함), 경과하면 1, 시작 전이면 0', () => {
    expect(timeFraction(day('2026-08-01'), day('2026-08-11'), day('2026-08-06'))).toBeCloseTo(5 / 11);
    expect(timeFraction(day('2026-08-01'), day('2026-08-11'), day('2026-09-01'))).toBe(1);
    expect(timeFraction(day('2026-08-01'), day('2026-08-11'), day('2026-07-01'))).toBe(0);
  });

  test('마감일 자정이 지나야 만충', () => {
    expect(timeFraction(day('2026-08-01'), day('2026-08-11'), day('2026-08-11'))).toBeCloseTo(10 / 11);
    expect(timeFraction(day('2026-08-01'), day('2026-08-11'), day('2026-08-12'))).toBe(1);
  });

  test('당일 마감 목표는 하루 동안 채워진다', () => {
    expect(timeFraction(day('2026-08-05'), day('2026-08-05'), day('2026-08-05'))).toBe(0);
    expect(timeFraction(day('2026-08-05'), day('2026-08-05'), day('2026-08-05').add(12, 'hour'))).toBeCloseTo(0.5);
  });

  test('생성 시점에 이미 지난 마감이면 1', () => {
    expect(timeFraction(day('2026-08-05'), day('2026-08-04'), day('2026-08-05'))).toBe(1);
  });
});

describe('dDayLabel', () => {
  test('D-n / D-DAY / D+n', () => {
    expect(dDayLabel(day('2026-08-08'), day('2026-08-05'))).toBe('D-3');
    expect(dDayLabel(day('2026-08-05'), day('2026-08-05'))).toBe('D-DAY');
    expect(dDayLabel(day('2026-08-03'), day('2026-08-05'))).toBe('D+2');
  });
});

describe('dueStatus', () => {
  const today = day('2026-08-05');

  test('미달 · 마감 전', () => {
    expect(dueStatus(1000, 10_000, day('2026-08-07'), today, 'full')).toEqual({ label: 'D-2 · 오늘 3,000자 필요', warning: false });
    expect(dueStatus(1000, 10_000, day('2026-08-07'), today, 'compact')).toEqual({ label: 'D-2 · 3,000자', warning: false });
  });

  test('미달 · 마감 당일', () => {
    expect(dueStatus(1000, 10_000, day('2026-08-05'), today, 'full')).toEqual({ label: 'D-DAY · 오늘 9,000자 필요', warning: false });
    expect(dueStatus(1000, 10_000, day('2026-08-05'), today, 'compact')).toEqual({ label: 'D-DAY · 9,000자', warning: false });
  });

  test('미달 · 마감 경과는 남은 글자 수와 경고', () => {
    expect(dueStatus(1000, 10_000, day('2026-08-03'), today, 'full')).toEqual({ label: 'D+2 · 9,000자 남음', warning: true });
    expect(dueStatus(1000, 10_000, day('2026-08-03'), today, 'compact')).toEqual({ label: 'D+2 · 9,000자 남음', warning: true });
  });

  test('달성 · 마감 전은 D-n만', () => {
    expect(dueStatus(10_000, 10_000, day('2026-08-07'), today, 'full')).toEqual({ label: 'D-2', warning: false });
    expect(dueStatus(10_000, 10_000, day('2026-08-07'), today, 'compact')).toEqual({ label: 'D-2', warning: false });
  });

  test('달성 · 마감 당일은 D-DAY만', () => {
    expect(dueStatus(10_000, 10_000, day('2026-08-05'), today, 'full')).toEqual({ label: 'D-DAY', warning: false });
    expect(dueStatus(10_000, 10_000, day('2026-08-05'), today, 'compact')).toEqual({ label: 'D-DAY', warning: false });
  });

  test('달성 · 마감 경과는 표기 생략', () => {
    expect(dueStatus(10_000, 10_000, day('2026-08-03'), today, 'full')).toBeNull();
    expect(dueStatus(10_000, 10_000, day('2026-08-03'), today, 'compact')).toBeNull();
  });
});

describe('pickGoalSource', () => {
  const goal = { targetCharacterCount: 1000 };

  test('자신의 목표가 있으면 자신의 현재값으로', () => {
    const entity = { id: 'e1', goal, ancestors: [{ id: 'a1', goal, node: { __typename: 'Folder', characterCount: 500 } }] };

    expect(pickGoalSource(entity, 300)).toEqual({ goal, current: 300, isFolder: false, entityId: 'e1' });
  });

  test('자신의 목표가 없으면 가장 가까운 조상 폴더 목표로', () => {
    const entity = {
      id: 'e1',
      goal: null,
      ancestors: [
        { id: 'a1', goal, node: { __typename: 'Folder', characterCount: 500 } },
        { id: 'a2', goal: null, node: { __typename: 'Folder', characterCount: 700 } },
      ],
    };

    expect(pickGoalSource(entity, 300)).toEqual({ goal, current: 500, isFolder: true, entityId: 'a1' });
  });

  test('목표가 하나도 없으면 null', () => {
    const entity = { id: 'e1', goal: null, ancestors: [{ id: 'a1', goal: null, node: { __typename: 'Folder', characterCount: 500 } }] };

    expect(pickGoalSource(entity, 300)).toBeNull();
  });
});
