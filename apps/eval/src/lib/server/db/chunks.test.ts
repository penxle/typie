import { describe, expect, it } from 'vitest';
import { chunkRows } from './index.ts';

const collect = (rows: number[], columns: number) => {
  const chunks: number[][] = [];
  chunkRows(rows, columns, (chunk) => {
    chunks.push(chunk);
  });
  return chunks;
};

describe('chunkRows', () => {
  // D1은 문장당 바인딩 100개까지다. 판정은 7컬럼이라 15행(105)에서 통째로 실패했다.
  it('컬럼 수 × 행 수가 100을 넘지 않게 나눈다', () => {
    const chunks = collect(
      Array.from({ length: 54 }, (_, i) => i),
      7,
    );
    expect(Math.max(...chunks.map((c) => c.length))).toBe(14);
    for (const chunk of chunks) expect(chunk.length * 7).toBeLessThanOrEqual(100);
  });

  it('모든 행이 정확히 한 번씩 들어간다', () => {
    const rows = Array.from({ length: 54 }, (_, i) => i);
    const chunks = collect(rows, 7);
    expect(chunks.flat()).toEqual(rows);
  });

  it('한 문장에 다 들어가면 한 번만 부른다', () => {
    expect(collect([1, 2, 3], 7)).toEqual([[1, 2, 3]]);
  });

  it('빈 배열이면 부르지 않는다', () => {
    expect(collect([], 7)).toEqual([]);
  });

  // 컬럼이 100을 넘어도 한 행씩은 보내야 한다 — 0으로 나누면 무한 루프가 된다.
  it('컬럼이 아주 많아도 한 행씩 보낸다', () => {
    expect(collect([1, 2], 200)).toEqual([[1], [2]]);
  });

  it('총평 판정은 6컬럼이라 16행까지 묶인다', () => {
    const chunks = collect(
      Array.from({ length: 20 }, (_, i) => i),
      6,
    );
    expect(chunks[0].length).toBe(16);
  });
});
