import { describe, expect, it } from 'vitest';
import { pickTask } from './assign.ts';

const tasks = (...ids: string[]) => ids.map((id) => ({ id }));

describe('pickTask', () => {
  it('후보 중에서 rng가 가리키는 것을 고른다', () => {
    expect(pickTask(tasks('a', 'b', 'c'), new Set(), new Set(), () => 0)).toBe('a');
    expect(pickTask(tasks('a', 'b', 'c'), new Set(), new Set(), () => 0.99)).toBe('c');
  });

  it('이미 배정된 태스크는 후보에서 빠진다', () => {
    expect(pickTask(tasks('a', 'b'), new Set(['a']), new Set(), () => 0)).toBe('b');
  });

  it('내가 반납한 태스크는 후보에서 빠진다', () => {
    expect(pickTask(tasks('a', 'b'), new Set(), new Set(['a']), () => 0)).toBe('b');
  });

  it('받을 것이 없으면 null', () => {
    expect(pickTask(tasks('a'), new Set(['a']), new Set())).toBeNull();
  });

  it('빈 목록이면 null', () => {
    expect(pickTask([], new Set(), new Set())).toBeNull();
  });

  it('기본 rng로도 후보 밖을 고르지 않는다', () => {
    for (let i = 0; i < 50; i++) {
      expect(pickTask(tasks('a', 'b', 'c'), new Set(['b']), new Set(['c']))).toBe('a');
    }
  });
});
