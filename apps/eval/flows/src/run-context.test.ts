import { describe, expect, it } from 'vitest';
import { emptyUsage } from '../../core/contracts.ts';
import { accumulate } from './run-context.ts';

describe('accumulate', () => {
  it('두 usage를 더한다', () => {
    const a = { ...emptyUsage(), calls: 1, promptTokens: 10, cachedTokens: 4 };
    const b = { ...emptyUsage(), calls: 2, promptTokens: 20, cacheWriteTokens: 3 };
    expect(accumulate(a, b)).toEqual({ calls: 3, promptTokens: 30, completionTokens: 0, cachedTokens: 4, cacheWriteTokens: 3 });
  });

  it('소수는 반올림한다', () => {
    expect(accumulate(emptyUsage(), { ...emptyUsage(), promptTokens: 1.4 }).promptTokens).toBe(1);
  });

  it('빈 usage끼리 더하면 빈 usage다', () => {
    expect(accumulate(emptyUsage(), emptyUsage())).toEqual(emptyUsage());
  });
});
