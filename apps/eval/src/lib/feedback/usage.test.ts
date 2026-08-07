import { describe, expect, it } from 'vitest';
import { sumUsage } from './usage.ts';
import type { RunUsage, UsageFold } from './types.ts';

const fold = (overrides: Partial<UsageFold>): UsageFold => ({
  provider: 'vendor',
  agent: 'agent',
  model: 'model',
  effort: null,
  turns: 1,
  inputTokens: 0,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheWriteTokens: 0,
  thinkingTokens: null,
  ...overrides,
});

describe('sumUsage', () => {
  it('folds의 네 축을 각각 합산한다', () => {
    const usage: RunUsage = {
      complete: true,
      folds: [
        fold({ inputTokens: 100, outputTokens: 20, cacheReadTokens: 3000, cacheWriteTokens: 400 }),
        fold({ inputTokens: 7, outputTokens: 8, cacheReadTokens: 9, cacheWriteTokens: 10 }),
        fold({ inputTokens: 1, outputTokens: 2, cacheReadTokens: 3, cacheWriteTokens: 4 }),
      ],
    };

    expect(sumUsage(usage)).toEqual({
      complete: true,
      inputTokens: 108,
      outputTokens: 30,
      cacheReadTokens: 3012,
      cacheWriteTokens: 414,
    });
  });

  it('thinkingTokens는 어떤 축에도 섞이지 않는다', () => {
    const usage: RunUsage = {
      complete: true,
      folds: [
        fold({ inputTokens: 10, outputTokens: 20, cacheReadTokens: 30, cacheWriteTokens: 40, thinkingTokens: 5000 }),
        fold({ inputTokens: 1, outputTokens: 2, cacheReadTokens: 3, cacheWriteTokens: 4, thinkingTokens: 6000 }),
      ],
    };

    expect(sumUsage(usage)).toEqual({
      complete: true,
      inputTokens: 11,
      outputTokens: 22,
      cacheReadTokens: 33,
      cacheWriteTokens: 44,
    });
  });

  it('complete false를 그대로 전파한다', () => {
    const usage: RunUsage = { complete: false, folds: [fold({ inputTokens: 5 })] };

    expect(sumUsage(usage)?.complete).toBe(false);
    expect(sumUsage(usage)?.inputTokens).toBe(5);
  });

  it('folds가 없으면 0으로 채운 합계를 낸다', () => {
    const usage = { complete: true } as unknown as RunUsage;

    expect(sumUsage(usage)).toEqual({
      complete: true,
      inputTokens: 0,
      outputTokens: 0,
      cacheReadTokens: 0,
      cacheWriteTokens: 0,
    });
  });

  it('usage가 없으면 null을 낸다', () => {
    expect(sumUsage(null)).toBeNull();
  });
});
