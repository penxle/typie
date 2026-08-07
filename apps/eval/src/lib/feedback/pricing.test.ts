import { describe, expect, it } from 'vitest';
import { DEFAULT_PRICE_TABLE, foldCost, formatKrw, sumCost } from './pricing.ts';
import type { PriceTable } from './pricing.ts';
import type { RunUsage, UsageFold } from './types.ts';

const fold = (overrides: Partial<UsageFold>): UsageFold => ({
  provider: 'anthropic',
  agent: 'agent',
  model: 'claude-opus-5',
  effort: null,
  turns: 1,
  inputTokens: 0,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheWriteTokens: 0,
  thinkingTokens: null,
  ...overrides,
});

describe('foldCost', () => {
  it('네 축을 각자의 단가로 쳐서 원화로 낸다', () => {
    // anthropic/claude-opus-5 = input $5 / output $25 / cacheRead $0.5 per 1M, usdKrw 1480.
    // 각 축 100만 토큰 → $5 + $25 + $0.5 = $30.5 → 30.5 × 1480 = 45,140원.
    const cost = foldCost(fold({ inputTokens: 1_000_000, outputTokens: 1_000_000, cacheReadTokens: 1_000_000 }));

    expect(cost?.krw).toBeCloseTo(45_140, 6);
  });

  it('캐시 쓰기는 입력가의 1.25배로 친다', () => {
    // 100만 토큰 × $5 × 1.25 = $6.25 → 6.25 × 1480 = 9,250원.
    const cost = foldCost(fold({ cacheWriteTokens: 1_000_000 }));

    expect(cost?.krw).toBeCloseTo(9250, 6);
  });

  it('표에 없는 모델이면 null을 낸다 — 다른 단가로 뭉뚱그리지 않는다', () => {
    expect(foldCost(fold({ provider: 'nowhere', model: 'unlisted', inputTokens: 1_000_000 }))).toBeNull();
  });

  it('provider와 model을 슬래시로 이어 키를 만든다', () => {
    expect(
      foldCost(fold({ provider: 'google-vertex-ai', model: 'google/gemini-3.5-flash-lite', inputTokens: 1_000_000 }))?.krw,
    ).toBeCloseTo(0.3 * 1480, 6);
    // 같은 model이라도 provider가 다르면 다른 키다.
    expect(foldCost(fold({ provider: 'google', model: 'google/gemini-3.5-flash-lite' }))).toBeNull();
  });

  it('cacheRead가 없는 단가는 입력가로 친다', () => {
    const table: PriceTable = { models: { 'x/y': { input: 4, output: 8 } }, usdKrw: 1000 };
    const cost = foldCost(fold({ provider: 'x', model: 'y', cacheReadTokens: 1_000_000 }), table);

    expect(cost?.krw).toBeCloseTo(4000, 6);
  });
});

describe('sumCost', () => {
  it('전 fold의 원가를 합산한다', () => {
    const usage: RunUsage = {
      complete: true,
      folds: [fold({ model: 'claude-opus-5', inputTokens: 1_000_000 }), fold({ model: 'claude-haiku-4-5', outputTokens: 1_000_000 })],
    };

    // $5 + $5 = $10 → 10 × 1480 = 14,800원.
    expect(sumCost(usage)?.krw).toBeCloseTo(14_800, 6);
  });

  it('단가 미상 fold가 하나라도 섞이면 부분합 대신 null을 낸다', () => {
    const usage: RunUsage = {
      complete: true,
      folds: [fold({ inputTokens: 1_000_000 }), fold({ provider: 'nowhere', model: 'unlisted', inputTokens: 1_000_000 })],
    };

    expect(sumCost(usage)).toBeNull();
  });

  it('folds가 없으면 0원을 낸다', () => {
    expect(sumCost({ complete: true } as unknown as RunUsage)).toEqual({ krw: 0 });
  });

  it('usage가 없으면 null을 낸다', () => {
    expect(sumCost(null)).toBeNull();
  });

  it('complete=false는 금액을 막지 않는다 — 그 값이 하한이라는 표식은 화면 몫이다', () => {
    const usage: RunUsage = { complete: false, folds: [fold({ inputTokens: 1_000_000 })] };

    expect(sumCost(usage)?.krw).toBeCloseTo(7400, 6);
  });
});

describe('formatKrw', () => {
  it('1만원 미만은 원 단위로 반올림한다', () => {
    expect(formatKrw(9250.4)).toBe('9,250원');
  });

  it('1만원 이상은 만원 단위로 접는다', () => {
    expect(formatKrw(45_140)).toBe('4.5만원');
  });
});

describe('DEFAULT_PRICE_TABLE', () => {
  it('승계한 모델 11종과 환율을 그대로 들고 있다', () => {
    expect(Object.keys(DEFAULT_PRICE_TABLE.models)).toHaveLength(11);
    expect(DEFAULT_PRICE_TABLE.usdKrw).toBe(1480);
  });
});
