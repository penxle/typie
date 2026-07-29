import { describe, expect, it } from 'vitest';
import { inChunks } from './db.ts';

describe('inChunks', () => {
  it('빈 목록이면 조회하지 않는다', async () => {
    let calls = 0;
    const out = await inChunks([], async (chunk) => {
      calls++;
      return chunk;
    });
    expect(out).toEqual([]);
    expect(calls).toBe(0);
  });

  it('90개까지는 한 번에 조회한다', async () => {
    const keys = Array.from({ length: 90 }, (_, i) => i);
    const sizes: number[] = [];
    await inChunks(keys, async (chunk) => {
      sizes.push(chunk.length);
      return chunk;
    });
    expect(sizes).toEqual([90]);
  });

  // D1은 문장당 바인딩 100개가 상한이다 — 넘기면 쿼리가 통째로 실패한다.
  it('상한을 넘는 목록은 쪼개서 조회하고 결과를 이어 붙인다', async () => {
    const keys = Array.from({ length: 1291 }, (_, i) => i);
    const sizes: number[] = [];
    const out = await inChunks(keys, async (chunk) => {
      sizes.push(chunk.length);
      return chunk;
    });
    expect(Math.max(...sizes)).toBeLessThanOrEqual(100);
    expect(sizes.reduce((a, b) => a + b, 0)).toBe(1291);
    expect(out).toEqual(keys);
  });
});
