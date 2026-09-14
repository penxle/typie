import { describe, expect, it } from 'vitest';
import { latestOfSpaceRuns } from './feed-grouping.ts';

const item = (id: string, spaceId: string) => ({ id, space: { id: spaceId } });

describe('latestOfSpaceRuns', () => {
  it('빈 목록은 빈 목록', () => {
    expect(latestOfSpaceRuns([])).toEqual([]);
  });

  it('같은 스페이스가 연속하면 첫 글만 남긴다', () => {
    const a1 = item('a1', 'A');
    const a2 = item('a2', 'A');
    const a3 = item('a3', 'A');
    const b1 = item('b1', 'B');
    const a4 = item('a4', 'A');
    expect(latestOfSpaceRuns([a1, a2, a3, b1, a4])).toEqual([a1, b1, a4]);
  });

  it('전부 같은 스페이스면 하나', () => {
    const items = [item('1', 'A'), item('2', 'A'), item('3', 'A')];
    expect(latestOfSpaceRuns(items)).toEqual([items[0]]);
  });

  it('앞 페이지 마지막 글과 같은 스페이스로 시작하면 이어서 접는다', () => {
    const a2 = item('a2', 'A');
    const b1 = item('b1', 'B');
    expect(latestOfSpaceRuns([a2, b1], 'A')).toEqual([b1]);
    expect(latestOfSpaceRuns([a2, b1], 'C')).toEqual([a2, b1]);
    expect(latestOfSpaceRuns([item('a3', 'A')], 'A')).toEqual([]);
  });

  it('입력을 바꾸지 않는다', () => {
    const items = [item('1', 'A'), item('2', 'A')];
    const copy = structuredClone(items);
    latestOfSpaceRuns(items);
    expect(items).toEqual(copy);
  });
});
