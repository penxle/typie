import { describe, expect, it } from 'vitest';
import { grepBefore, mergeRanges, readRanges, uncovered, withinRead } from './ledger.ts';
import type { ToolRecord } from './ledger.ts';

const READS: ToolRecord[] = [
  { turn: 0, tool: 'read', start: 0, end: 100 },
  { turn: 1, tool: 'read', start: 80, end: 200 },
  { turn: 3, tool: 'grep', pattern: '광역', total: 2 },
];

describe('mergeRanges', () => {
  it('겹치는 범위를 병합·정렬한다', () => {
    expect(
      mergeRanges([
        { start: 80, end: 200 },
        { start: 0, end: 100 },
      ]),
    ).toEqual([{ start: 0, end: 200 }]);
  });
});

describe('uncovered', () => {
  it('미열람 구간을 돌려준다', () => {
    expect(uncovered(300, readRanges(READS), [])).toEqual([{ start: 200, end: 300 }]);
  });

  // 후기 등 제외 구간은 커버리지 의무에서 뺀다.
  it('제외 구간은 미열람으로 치지 않는다', () => {
    expect(uncovered(300, readRanges(READS), [{ start: 200, end: 300 }])).toEqual([]);
  });
});

describe('withinRead', () => {
  it('열람 범위 안 인용만 허용한다', () => {
    expect(withinRead(READS, 150, 180)).toBe(true);
    expect(withinRead(READS, 190, 250)).toBe(false);
  });
});

describe('grepBefore', () => {
  it('해당 턴 이전의 grep 존재를 판정한다', () => {
    expect(grepBefore(READS, 4)).toBe(true);
    expect(grepBefore(READS, 3)).toBe(false);
  });
});
