import { describe, expect, it } from 'vitest';
import { summarizeAgentCosts, synthesizeFolds, usageLowerBound } from './stage-cost.ts';
import type { AgentUsage } from './live.ts';
import type { PriceTable } from './pricing.ts';
import type { ModelConfig } from './tiers.ts';
import type { UsageFold } from './types.ts';

// 단가 픽스처(실표는 prism이 정본) — opus 입력 $5/M, sonnet 입력 $2/M, 환율 1480.
const TABLE: PriceTable = {
  models: {
    'anthropic/claude-opus-5': { input: 5, output: 25, cacheRead: 0.5, cacheWrite: 10 },
    'anthropic/claude-sonnet-5': { input: 2, output: 10, cacheRead: 0.2, cacheWrite: 4 },
  },
  usdKrw: 1480,
};
const fold = (over: Partial<UsageFold>): UsageFold => ({
  provider: 'anthropic',
  agent: 'judgment-high',
  model: 'claude-opus-5',
  effort: 'xhigh',
  turns: 1,
  inputTokens: 1_000_000,
  outputTokens: 0,
  cacheReadTokens: 0,
  cacheWriteTokens: 0,
  thinkingTokens: null,
  ...over,
});

describe('synthesizeFolds', () => {
  const totals: AgentUsage = { turns: 3, inputTokens: 1000, outputTokens: 100, cacheReadTokens: 10, cacheWriteTokens: 1 };
  const config: ModelConfig = { 'judgment-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'xhigh', overridden: false } };

  it('modelConfig로 provider·model을 입힌다 — 에이전트 이름 그대로 조회한다', () => {
    expect(synthesizeFolds({ 'judgment-high': totals }, config)).toEqual([
      {
        provider: 'anthropic',
        agent: 'judgment-high',
        model: 'claude-opus-5',
        effort: 'xhigh',
        turns: 3,
        inputTokens: 1000,
        outputTokens: 100,
        cacheReadTokens: 10,
        cacheWriteTokens: 1,
        thinkingTokens: null,
      },
    ]);
  });

  it('스냅샷에 없는 에이전트는 빈 provider·model로 남는다 — foldCost가 null을 내는 형태', () => {
    const [synthesized] = synthesizeFolds({ 'judgment-high': totals }, null);
    expect(synthesized.provider).toBe('');
    expect(synthesized.model).toBe('');
  });

  it('컷오버 전 스냅샷(provider 부재)은 빈 provider로 남는다 — 미상 강등', () => {
    const [synthesized] = synthesizeFolds(
      { 'judgment-high': totals },
      { 'judgment-high': { model: 'claude-opus-5', effort: 'xhigh', overridden: false } },
    );
    expect(synthesized.provider).toBe('');
    expect(synthesized.model).toBe('claude-opus-5');
  });
});

describe('summarizeAgentCosts', () => {
  it('fold를 base 이름으로 접는다 — 재리뷰 -followup은 본 행에 합산된다', () => {
    const summary = summarizeAgentCosts(
      [fold({ agent: 'judgment-high' }), fold({ agent: 'judgment-high-followup', model: 'claude-sonnet-5', inputTokens: 500_000 })],
      ['judgment-high'],
      TABLE,
    );
    expect(summary.agents['judgment-high']).toBeCloseTo(8880, 5);
    expect(summary.etc).toBeUndefined();
    expect(summary.total).toBeCloseTo(8880, 5);
  });

  it('알려진 에이전트 밖 fold는 기타로 접고 합계에는 포함한다', () => {
    const summary = summarizeAgentCosts([fold({ agent: 'manuscript' })], ['judgment-high'], TABLE);
    expect(summary.agents).toEqual({});
    expect(summary.etc).toBeCloseTo(7400, 5);
    expect(summary.total).toBeCloseTo(7400, 5);
  });

  it('단가 미상 fold는 그 행과 합계를 함께 미상으로 만든다 — 다른 행은 오염시키지 않는다', () => {
    const summary = summarizeAgentCosts(
      [fold({ agent: 'judgment-high', provider: '', model: '' }), fold({ agent: 'stylistic-high' })],
      ['judgment-high', 'stylistic-high'],
      TABLE,
    );
    expect(summary.agents['judgment-high']).toBeNull();
    expect(summary.agents['stylistic-high']).toBeCloseTo(7400, 5);
    expect(summary.total).toBeNull();
  });

  it('fold가 없으면 합계 0이다', () => {
    expect(summarizeAgentCosts([], ['judgment-high'], TABLE)).toEqual({ agents: {}, total: 0 });
  });

  it('표가 없으면(수신 실패) 전 행과 합계가 미상이다', () => {
    const summary = summarizeAgentCosts([fold({ agent: 'judgment-high' })], ['judgment-high'], null);
    expect(summary.agents['judgment-high']).toBeNull();
    expect(summary.total).toBeNull();
  });
});

describe('usageLowerBound', () => {
  it('complete:false만 하한이다 — 부재는 하한이 아니라 기록 없음', () => {
    expect(usageLowerBound(null)).toBe(false);
    expect(usageLowerBound({ complete: false, folds: [] })).toBe(true);
    expect(usageLowerBound({ complete: true, folds: [] })).toBe(false);
  });
});
