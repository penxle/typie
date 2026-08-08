import { describe, expect, it } from 'vitest';
import { AGENT_DEFAULTS, buildModelConfig, resolveTierSubmission } from './tiers.ts';

describe('buildModelConfig', () => {
  it('무오버라이드는 전 에이전트 기본값에 overridden:false다', () => {
    const config = buildModelConfig(undefined);
    expect(config.research).toEqual({ model: 'claude-opus-5', effort: 'xhigh', overridden: false });
    expect(Object.values(config)).toHaveLength(7);
    expect(Object.values(config).every((entry) => !entry.overridden)).toBe(true);
  });

  it('오버라이드 항목만 overridden:true로 대체된다', () => {
    const config = buildModelConfig({ review: { model: 'claude-sonnet-5', effort: 'xhigh' } });
    expect(config.review).toEqual({ model: 'claude-sonnet-5', effort: 'xhigh', overridden: true });
    expect(config.plan).toEqual({ ...AGENT_DEFAULTS.plan, overridden: false });
  });
});

describe('resolveTierSubmission', () => {
  it('빈 제출은 non-admin도 통과한다', () => {
    expect(resolveTierSubmission({}, false)).toEqual({ overrides: {} });
  });

  it('non-admin의 티어 제출은 거절된다', () => {
    expect(resolveTierSubmission({ review: { model: 'claude-sonnet-5', effort: 'xhigh' } }, false)).toHaveProperty('error');
  });

  it('미지 에이전트·미지 모델·무효 effort 조합을 거절한다', () => {
    expect(resolveTierSubmission({ editor: { model: 'claude-opus-5', effort: 'high' } }, true)).toHaveProperty('error');
    expect(resolveTierSubmission({ review: { model: 'gpt-6', effort: 'high' } }, true)).toHaveProperty('error');
    expect(resolveTierSubmission({ review: { model: 'claude-fable-5', effort: 'none' } }, true)).toHaveProperty('error');
    expect(resolveTierSubmission({ research: { model: 'claude-opus-5', effort: 'none' } }, true)).toHaveProperty('error');
    expect(resolveTierSubmission({ review: { model: 'gemini-3.6-flash', effort: 'xhigh' } }, true)).toHaveProperty('error');
  });

  it('gemini 모델은 thinkingLevel 어휘를 수용한다', () => {
    expect(resolveTierSubmission({ proofread: { model: 'gemini-3.5-flash-lite', effort: 'minimal' } }, true)).toEqual({
      overrides: { proofread: { model: 'gemini-3.5-flash-lite', effort: 'minimal' } },
    });
  });

  it('기본값과 같은 명시 선택은 no-op이다', () => {
    expect(resolveTierSubmission({ review: { model: 'gpt-5.6-sol', effort: 'high' } }, true)).toEqual({ overrides: {} });
  });

  it('유효 오버라이드를 수용한다', () => {
    expect(resolveTierSubmission({ proofread: { model: 'gpt-5.6-luna', effort: 'low' } }, true)).toEqual({
      overrides: { proofread: { model: 'gpt-5.6-luna', effort: 'low' } },
    });
  });
});
