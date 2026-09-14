import { describe, expect, it } from 'vitest';
import { groupConsecutiveBySpace } from './feed-grouping.ts';

const item = (id: string, spaceId: string) => ({ id, space: { id: spaceId } });

describe('groupConsecutiveBySpace', () => {
  it('빈 목록은 빈 그룹', () => {
    expect(groupConsecutiveBySpace([])).toEqual([]);
  });

  it('같은 스페이스가 연속하면 첫 글만 남기고 나머지를 센다', () => {
    const a1 = item('a1', 'A');
    const a2 = item('a2', 'A');
    const a3 = item('a3', 'A');
    const b1 = item('b1', 'B');
    const a4 = item('a4', 'A');
    expect(groupConsecutiveBySpace([a1, a2, a3, b1, a4])).toEqual([
      { lead: a1, more: 2 },
      { lead: b1, more: 0 },
      { lead: a4, more: 0 },
    ]);
  });

  it('전부 같은 스페이스면 그룹 하나', () => {
    const items = [item('1', 'A'), item('2', 'A'), item('3', 'A')];
    expect(groupConsecutiveBySpace(items)).toEqual([{ lead: items[0], more: 2 }]);
  });

  it('입력을 바꾸지 않는다', () => {
    const items = [item('1', 'A'), item('2', 'A')];
    const copy = structuredClone(items);
    groupConsecutiveBySpace(items);
    expect(items).toEqual(copy);
  });
});
