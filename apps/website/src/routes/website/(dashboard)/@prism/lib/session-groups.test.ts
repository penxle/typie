import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';
import { groupSessionsByRecency, matchesSessionQuery } from './session-groups.ts';

const now = dayjs('2026-08-21T15:00:00+09:00');
const at = (iso: string) => ({ updatedAt: dayjs(iso).toISOString() });

describe('groupSessionsByRecency', () => {
  it('오늘·어제·지난 7일·지난 30일·월 순으로 묶고 그룹 안은 최근순이다', () => {
    const sessions = [
      { id: 'old', ...at('2026-06-03T10:00:00+09:00') },
      { id: 'today-early', ...at('2026-08-21T01:00:00+09:00') },
      { id: 'week', ...at('2026-08-16T12:00:00+09:00') },
      { id: 'today-late', ...at('2026-08-21T14:59:00+09:00') },
      { id: 'yesterday', ...at('2026-08-20T23:30:00+09:00') },
      { id: 'month', ...at('2026-07-30T09:00:00+09:00') },
      { id: 'last-year', ...at('2025-12-25T09:00:00+09:00') },
    ];
    const groups = groupSessionsByRecency(sessions, now);
    expect(groups.map((g) => [g.label, g.sessions.map((s) => s.id)])).toEqual([
      ['오늘', ['today-late', 'today-early']],
      ['어제', ['yesterday']],
      ['지난 7일', ['week']],
      ['지난 30일', ['month']],
      ['6월', ['old']],
      ['2025년 12월', ['last-year']],
    ]);
  });

  it('경계: 자정 직후는 오늘, 7일 전 자정은 지난 7일, 그 1초 전은 지난 30일이다', () => {
    const groups = groupSessionsByRecency(
      [
        { id: 'a', ...at('2026-08-21T00:00:00+09:00') },
        { id: 'b', ...at('2026-08-14T00:00:00+09:00') },
        { id: 'c', ...at('2026-08-13T23:59:59+09:00') },
      ],
      now,
    );
    expect(groups.map((g) => [g.label, g.sessions.map((s) => s.id)])).toEqual([
      ['오늘', ['a']],
      ['지난 7일', ['b']],
      ['지난 30일', ['c']],
    ]);
  });

  it('빈 입력은 빈 그룹이다', () => {
    expect(groupSessionsByRecency([], now)).toEqual([]);
  });
});

describe('matchesSessionQuery', () => {
  it('빈 질의는 전부, 아니면 제목 부분 일치(대소문자 무시)다', () => {
    expect(matchesSessionQuery('추리 트릭 검증', '')).toBe(true);
    expect(matchesSessionQuery('추리 트릭 검증', ' 트릭 ')).toBe(true);
    expect(matchesSessionQuery('Plot Holes', 'plot')).toBe(true);
    expect(matchesSessionQuery(null, 'x')).toBe(false);
  });
});
