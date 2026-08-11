import { describe, expect, it } from 'vitest';
import { foldCost, formatKrw, sumCost } from './pricing.ts';
import type { PriceTable } from './pricing.ts';
import type { RunUsage, UsageFold } from './types.ts';

// 실표는 prism이 정본이라 여기 없다 — 이 픽스처는 형태와 환산 규칙만 고정한다(단가는 이관 시점의 실값 발췌).
const TABLE: PriceTable = {
  models: {
    'anthropic/claude-opus-5': { input: 5, output: 25, cacheRead: 0.5, cacheWrite: 10 },
    'anthropic/claude-haiku-4-5': { input: 1, output: 5, cacheRead: 0.1, cacheWrite: 2 },
    'openai/gpt-5.6-sol': { input: 5, output: 30, cacheRead: 0.5, cacheWrite: 6.25 },
    'google-vertex-ai/google/gemini-3.5-flash-lite': { input: 0.3, output: 2.5, cacheRead: 0.03, cacheWrite: 0.375 },
  },
  usdKrw: 1480,
};

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
    const cost = foldCost(fold({ inputTokens: 1_000_000, outputTokens: 1_000_000, cacheReadTokens: 1_000_000 }), TABLE);

    expect(cost?.krw).toBeCloseTo(45_140, 6);
  });

  it('캐시 쓰기는 표의 cacheWrite 단가로 친다 — TTL 승수는 정본에서 이미 접혀 온다', () => {
    // anthropic/claude-opus-5: 100만 토큰 × $10 = $10 → 10 × 1480 = 14,800원.
    expect(foldCost(fold({ cacheWriteTokens: 1_000_000 }), TABLE)?.krw).toBeCloseTo(14_800, 6);

    // openai/gpt-5.6-sol: 100만 토큰 × $6.25 = $6.25 → 6.25 × 1480 = 9,250원.
    expect(foldCost(fold({ provider: 'openai', model: 'gpt-5.6-sol', cacheWriteTokens: 1_000_000 }), TABLE)?.krw).toBeCloseTo(9250, 6);
  });

  it('표에 없는 모델이면 null을 낸다 — 다른 단가로 뭉뚱그리지 않는다', () => {
    expect(foldCost(fold({ provider: 'nowhere', model: 'unlisted', inputTokens: 1_000_000 }), TABLE)).toBeNull();
  });

  it('표가 없으면(수신 실패) null을 낸다 — 금액을 지어내지 않는다', () => {
    expect(foldCost(fold({ inputTokens: 1_000_000 }), null)).toBeNull();
  });

  it('provider와 model을 슬래시로 이어 키를 만든다', () => {
    expect(
      foldCost(fold({ provider: 'google-vertex-ai', model: 'google/gemini-3.5-flash-lite', inputTokens: 1_000_000 }), TABLE)?.krw,
    ).toBeCloseTo(0.3 * 1480, 6);
    // 같은 model이라도 provider가 다르면 다른 키다.
    expect(foldCost(fold({ provider: 'google', model: 'google/gemini-3.5-flash-lite' }), TABLE)).toBeNull();
  });

  it('cacheRead가 없는 단가는 입력가로 친다', () => {
    const table: PriceTable = { models: { 'x/y': { input: 4, output: 8, cacheWrite: 5 } }, usdKrw: 1000 };
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
    expect(sumCost(usage, TABLE)?.krw).toBeCloseTo(14_800, 6);
  });

  it('단가 미상 fold가 하나라도 섞이면 부분합 대신 null을 낸다', () => {
    const usage: RunUsage = {
      complete: true,
      folds: [fold({ inputTokens: 1_000_000 }), fold({ provider: 'nowhere', model: 'unlisted', inputTokens: 1_000_000 })],
    };

    expect(sumCost(usage, TABLE)).toBeNull();
  });

  it('표가 없으면(수신 실패) 소비가 있는 usage는 null이다', () => {
    const usage: RunUsage = { complete: true, folds: [fold({ inputTokens: 1_000_000 })] };

    expect(sumCost(usage, null)).toBeNull();
  });

  it('folds가 없으면 0원을 낸다', () => {
    expect(sumCost({ complete: true } as unknown as RunUsage, TABLE)).toEqual({ krw: 0 });
  });

  it('usage가 없으면 null을 낸다', () => {
    expect(sumCost(null, TABLE)).toBeNull();
  });

  it('complete=false는 금액을 막지 않는다 — 그 값이 하한이라는 표식은 화면 몫이다', () => {
    const usage: RunUsage = { complete: false, folds: [fold({ inputTokens: 1_000_000 })] };

    expect(sumCost(usage, TABLE)?.krw).toBeCloseTo(7400, 6);
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
