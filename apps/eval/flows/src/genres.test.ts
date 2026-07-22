import { describe, expect, it } from 'vitest';
import { allocateByLargestRemainder, normalizeGenre } from './genres.ts';

describe('normalizeGenre', () => {
  it('목록 밖 값은 etc로 정규화한다', () => {
    expect(normalizeGenre('romance')).toBe('romance');
    expect(normalizeGenre('무협')).toBe('etc');
    expect(normalizeGenre('')).toBe('etc');
  });
});

describe('allocateByLargestRemainder', () => {
  it('비례 배분 합계가 size와 같다', () => {
    const alloc = allocateByLargestRemainder({ romance: 50, fantasy: 30, etc: 20 }, 10);
    expect(alloc).toEqual({ romance: 5, fantasy: 3, etc: 2 });
  });

  it('잔여가 큰 층부터 올림 배분한다', () => {
    const alloc = allocateByLargestRemainder({ a: 1, b: 1, c: 1 }, 2);
    expect(Object.values(alloc).reduce((s, n) => s + n, 0)).toBe(2);
    expect(Object.values(alloc).every((n) => n <= 1)).toBe(true);
  });

  it('size가 풀보다 크면 풀 전체를 배분한다', () => {
    const alloc = allocateByLargestRemainder({ a: 2, b: 1 }, 10);
    expect(alloc).toEqual({ a: 2, b: 1 });
  });

  it('층 배분량은 층 크기를 넘지 않는다', () => {
    const alloc = allocateByLargestRemainder({ a: 9, b: 1 }, 5);
    expect(alloc.b).toBeLessThanOrEqual(1);
    expect(Object.values(alloc).reduce((s, n) => s + n, 0)).toBe(5);
  });
});
