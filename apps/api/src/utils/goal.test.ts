import '@typie/lib/dayjs';

import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import dayjs from 'dayjs';
import { buildDailyHistory, getEffectiveTarget } from './goal.ts';

const day = (s: string) => dayjs.kst(s).startOf('day');

describe('getEffectiveTarget', () => {
  test('빈 이력이면 null', () => {
    assert.equal(getEffectiveTarget([], day('2026-08-05')), null);
  });

  test('가장 최근 유효 행의 목표를 반환', () => {
    const entries = [
      { targetCharacterCount: 500, effectiveAt: day('2026-08-01') },
      { targetCharacterCount: 1000, effectiveAt: day('2026-08-03') },
    ];
    assert.equal(getEffectiveTarget(entries, day('2026-08-02')), 500);
    assert.equal(getEffectiveTarget(entries, day('2026-08-03')), 1000);
    assert.equal(getEffectiveTarget(entries, day('2026-08-05')), 1000);
  });

  test('시작 전 날짜는 null', () => {
    const entries = [{ targetCharacterCount: 500, effectiveAt: day('2026-08-01') }];
    assert.equal(getEffectiveTarget(entries, day('2026-07-31')), null);
  });

  test('해제 마커(null) 이후는 null, 재설정하면 다시 유효', () => {
    const entries = [
      { targetCharacterCount: 500, effectiveAt: day('2026-08-01') },
      { targetCharacterCount: null, effectiveAt: day('2026-08-03') },
      { targetCharacterCount: 700, effectiveAt: day('2026-08-05') },
    ];
    assert.equal(getEffectiveTarget(entries, day('2026-08-02')), 500);
    assert.equal(getEffectiveTarget(entries, day('2026-08-04')), null);
    assert.equal(getEffectiveTarget(entries, day('2026-08-06')), 700);
  });
});

describe('buildDailyHistory', () => {
  test('빈 입력이면 빈 배열', () => {
    assert.deepEqual(buildDailyHistory([], '2026-08-05'), []);
  });

  test('단일 문서: 관측일 사이와 until까지 forward-fill', () => {
    const rows = [
      { documentId: 'D1', date: '2026-08-01', characterCount: 100 },
      { documentId: 'D1', date: '2026-08-03', characterCount: 250 },
    ];
    assert.deepEqual(buildDailyHistory(rows, '2026-08-04'), [
      { date: '2026-08-01', characterCount: 100 },
      { date: '2026-08-02', characterCount: 100 },
      { date: '2026-08-03', characterCount: 250 },
      { date: '2026-08-04', characterCount: 250 },
    ]);
  });

  test('다중 문서: 늦게 등장한 문서는 등장일부터 합산', () => {
    const rows = [
      { documentId: 'D1', date: '2026-08-01', characterCount: 100 },
      { documentId: 'D2', date: '2026-08-02', characterCount: 40 },
      { documentId: 'D1', date: '2026-08-03', characterCount: 150 },
    ];
    assert.deepEqual(buildDailyHistory(rows, '2026-08-03'), [
      { date: '2026-08-01', characterCount: 100 },
      { date: '2026-08-02', characterCount: 140 },
      { date: '2026-08-03', characterCount: 190 },
    ]);
  });
});
