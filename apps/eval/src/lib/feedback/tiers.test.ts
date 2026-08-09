import { describe, expect, it } from 'vitest';
import { AGENT_DEFAULTS, buildModelConfig, resolveTierSubmission, TIER_AGENTS, TIER_NAMES } from './tiers.ts';

describe('TIER_AGENTS', () => {
  it('티어별 에이전트가 기본값 목록과 정확히 맞물린다', () => {
    const listed = TIER_NAMES.flatMap((tier) => TIER_AGENTS[tier]);
    for (const agent of listed) {
      expect(AGENT_DEFAULTS[agent]).toBeDefined();
    }
    expect(new Set(listed).size).toBe(listed.length);
    expect(new Set(listed)).toEqual(new Set(Object.keys(AGENT_DEFAULTS)));
  });
});

describe('buildModelConfig', () => {
  it('low는 세 에이전트만 담고 전부 기본값이다', () => {
    const config = buildModelConfig('low', undefined);
    expect(Object.keys(config)).toEqual(['critique-low', 'proofread-low', 'rephrase-low']);
    expect(config['critique-low']).toEqual({ model: 'gemini-3.6-flash', effort: 'high', overridden: false });
    expect(Object.values(config).every((entry) => !entry?.overridden)).toBe(true);
  });

  it('high는 일곱 에이전트를 담고 오버라이드 항목만 overridden:true다', () => {
    const config = buildModelConfig('high', { 'review-high': { model: 'claude-sonnet-5', effort: 'xhigh' } });
    expect(Object.keys(config)).toHaveLength(7);
    expect(config['review-high']).toEqual({ model: 'claude-sonnet-5', effort: 'xhigh', overridden: true });
    expect(config['plan-high']).toEqual({ ...AGENT_DEFAULTS['plan-high'], overridden: false });
  });

  it('다른 티어의 오버라이드는 담기지 않는다', () => {
    const config = buildModelConfig('medium', { 'review-high': { model: 'claude-sonnet-5', effort: 'xhigh' } });
    expect(Object.keys(config)).toEqual(['research-medium', 'critique-medium', 'proofread-medium', 'rephrase-medium', 'conclude-medium']);
  });
});

describe('resolveTierSubmission', () => {
  it('티어 판정이 운영자 판정보다 앞선다', () => {
    expect(resolveTierSubmission('extreme', {}, false)).toEqual({ error: '알 수 없는 티어예요: extreme' });
  });

  it('non-admin은 high 무오버라이드만 통과한다', () => {
    expect(resolveTierSubmission('high', {}, false)).toEqual({ tier: 'high', overrides: {} });
    expect(resolveTierSubmission('medium', {}, false)).toEqual({ error: '티어 설정은 운영자만 쓸 수 있어요' });
    expect(resolveTierSubmission('low', {}, false)).toHaveProperty('error');
    expect(resolveTierSubmission('high', { 'review-high': { model: 'claude-sonnet-5', effort: 'xhigh' } }, false)).toHaveProperty('error');
  });

  it('admin은 medium·low를 수용한다', () => {
    expect(resolveTierSubmission('medium', {}, true)).toEqual({ tier: 'medium', overrides: {} });
    expect(resolveTierSubmission('low', { 'proofread-low': { model: 'gemini-3.5-flash-lite', effort: 'minimal' } }, true)).toEqual({
      tier: 'low',
      overrides: { 'proofread-low': { model: 'gemini-3.5-flash-lite', effort: 'minimal' } },
    });
  });

  it('해당 티어에 없는 에이전트를 거절한다', () => {
    expect(resolveTierSubmission('low', { 'research-medium': { model: 'claude-sonnet-5', effort: 'medium' } }, true)).toEqual({
      error: '이 티어에 없는 에이전트예요: research-medium',
    });
    expect(resolveTierSubmission('high', { editor: { model: 'claude-opus-5', effort: 'high' } }, true)).toHaveProperty('error');
  });

  it('미지 모델·무효 effort를 거절한다', () => {
    expect(resolveTierSubmission('high', { 'review-high': { model: 'gpt-6', effort: 'high' } }, true)).toHaveProperty('error');
    expect(resolveTierSubmission('high', { 'research-high': { model: 'claude-opus-5', effort: 'none' } }, true)).toHaveProperty('error');
    expect(resolveTierSubmission('low', { 'critique-low': { model: 'gemini-3.6-flash', effort: 'xhigh' } }, true)).toHaveProperty('error');
  });

  it('기본값과 같은 명시 선택은 no-op이다', () => {
    expect(resolveTierSubmission('high', { 'review-high': { model: 'gpt-5.6-sol', effort: 'xhigh' } }, true)).toEqual({
      tier: 'high',
      overrides: {},
    });
    expect(resolveTierSubmission('medium', { 'critique-medium': { model: 'claude-sonnet-5', effort: 'high' } }, true)).toEqual({
      tier: 'medium',
      overrides: {},
    });
  });

  it('유효 오버라이드를 수용한다', () => {
    expect(resolveTierSubmission('medium', { 'rephrase-medium': { model: 'gpt-5.6-luna', effort: 'low' } }, true)).toEqual({
      tier: 'medium',
      overrides: { 'rephrase-medium': { model: 'gpt-5.6-luna', effort: 'low' } },
    });
  });
});
