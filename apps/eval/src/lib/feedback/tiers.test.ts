import { describe, expect, it } from 'vitest';
import { buildModelConfig, overridesFromConfig, resolveTierSubmission } from './tiers.ts';
import type { AppCatalog } from './tiers.ts';

// 카탈로그 픽스처 — prism 카탈로그 표면의 형태 재현이다. 값의 정본 동기화 의무는 없다(기능 검증용).
const CATALOG = {
  models: {
    'claude-opus-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
    'claude-sonnet-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
    'gpt-5.6-luna': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
    'gemini-3.6-flash': { provider: 'gemini', efforts: ['minimal', 'low', 'medium', 'high'] },
    'gemini-3.5-flash-lite': { provider: 'gemini', efforts: ['minimal', 'low', 'medium', 'high'] },
    'deepseek-v4-flash': { provider: 'deepseek', efforts: ['medium'] },
  },
  agents: {
    'description-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'interpretation-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'rubric-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'calibration-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'judgment-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'stylistic-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'delivery-high': { provider: 'anthropic', model: 'claude-opus-5', effort: 'high' },
    'research-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'critique-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'high' },
    'proofread-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'high' },
    'rephrase-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'conclude-medium': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'medium' },
    'critique-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
    'proofread-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
    'rephrase-low': { provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high' },
  },
  workflows: {
    high: {
      agents: [
        'description-high',
        'interpretation-high',
        'rubric-high',
        'calibration-high',
        'judgment-high',
        'stylistic-high',
        'delivery-high',
      ],
    },
    medium: { agents: ['research-medium', 'critique-medium', 'proofread-medium', 'rephrase-medium', 'conclude-medium'] },
    low: { agents: ['critique-low', 'proofread-low', 'rephrase-low'] },
  },
} satisfies AppCatalog;

describe('buildModelConfig', () => {
  it('low는 세 에이전트만 담고 전부 기본값이다 — provider까지 스냅샷에 실린다', () => {
    const config = buildModelConfig(CATALOG, 'low', undefined);
    expect(Object.keys(config)).toEqual(['critique-low', 'proofread-low', 'rephrase-low']);
    expect(config['critique-low']).toEqual({ provider: 'gemini', model: 'gemini-3.6-flash', effort: 'high', overridden: false });
    expect(Object.values(config).every((entry) => !entry?.overridden)).toBe(true);
  });

  it('high는 일곱 에이전트를 담고 오버라이드 항목만 overridden:true다', () => {
    const config = buildModelConfig(CATALOG, 'high', {
      'calibration-high': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'xhigh' },
    });
    expect(Object.keys(config)).toHaveLength(7);
    expect(config['calibration-high']).toEqual({ provider: 'anthropic', model: 'claude-sonnet-5', effort: 'xhigh', overridden: true });
    expect(config['rubric-high']).toEqual({ provider: 'anthropic', model: 'claude-opus-5', effort: 'high', overridden: false });
  });

  it('다른 티어의 오버라이드는 담기지 않는다', () => {
    const config = buildModelConfig(CATALOG, 'medium', {
      'calibration-high': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'xhigh' },
    });
    expect(Object.keys(config)).toEqual(['research-medium', 'critique-medium', 'proofread-medium', 'rephrase-medium', 'conclude-medium']);
  });
});

describe('overridesFromConfig', () => {
  it('overridden 항목만 뽑아 3장 그대로 되돌린다', () => {
    const overrides = { 'calibration-high': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'xhigh' } };
    expect(overridesFromConfig(CATALOG, buildModelConfig(CATALOG, 'high', overrides))).toEqual(overrides);
  });

  it('전부 기본값이거나 스냅샷이 없으면 빈 객체다', () => {
    expect(overridesFromConfig(CATALOG, buildModelConfig(CATALOG, 'high', undefined))).toEqual({});
    expect(overridesFromConfig(CATALOG, null)).toEqual({});
  });

  it('컷오버 전 스냅샷(provider 부재)은 카탈로그로 채우고, 목록 밖 모델은 빈 provider로 명시 실패에 맡긴다', () => {
    const legacy = {
      'calibration-high': { model: 'claude-sonnet-5', effort: 'xhigh', overridden: true },
      'judgment-high': { model: 'gpt-4-retired', effort: 'high', overridden: true },
    };
    expect(overridesFromConfig(CATALOG, legacy)).toEqual({
      'calibration-high': { provider: 'anthropic', model: 'claude-sonnet-5', effort: 'xhigh' },
      'judgment-high': { provider: '', model: 'gpt-4-retired', effort: 'high' },
    });
  });
});

describe('resolveTierSubmission', () => {
  it('티어 판정이 운영자 판정보다 앞선다', () => {
    expect(resolveTierSubmission(CATALOG, 'extreme', {}, false)).toEqual({ error: '알 수 없는 티어예요: extreme' });
  });

  it('화면이 아는 티어라도 카탈로그에 없으면 반려한다', () => {
    const partial = { ...CATALOG, workflows: { high: CATALOG.workflows.high } };
    expect(resolveTierSubmission(partial, 'medium', {}, true)).toEqual({ error: '카탈로그에 없는 티어예요: medium' });
  });

  it('non-admin은 전 티어 무오버라이드만 통과한다 — 오버라이드 항목은 운영자 전용', () => {
    expect(resolveTierSubmission(CATALOG, 'high', {}, false)).toEqual({ tier: 'high', overrides: {} });
    expect(resolveTierSubmission(CATALOG, 'medium', {}, false)).toEqual({ tier: 'medium', overrides: {} });
    expect(resolveTierSubmission(CATALOG, 'low', {}, false)).toEqual({ tier: 'low', overrides: {} });
    expect(
      resolveTierSubmission(CATALOG, 'high', { 'calibration-high': { model: 'claude-sonnet-5', effort: 'xhigh' } }, false),
    ).toHaveProperty('error');
    expect(
      resolveTierSubmission(CATALOG, 'low', { 'critique-low': { model: 'gemini-3.5-flash-lite', effort: 'minimal' } }, false),
    ).toHaveProperty('error');
  });

  it('admin은 medium·low를 수용하고 오버라이드에 provider를 채운다', () => {
    expect(resolveTierSubmission(CATALOG, 'medium', {}, true)).toEqual({ tier: 'medium', overrides: {} });
    expect(resolveTierSubmission(CATALOG, 'low', { 'proofread-low': { model: 'gemini-3.5-flash-lite', effort: 'minimal' } }, true)).toEqual(
      {
        tier: 'low',
        overrides: { 'proofread-low': { provider: 'gemini', model: 'gemini-3.5-flash-lite', effort: 'minimal' } },
      },
    );
    expect(resolveTierSubmission(CATALOG, 'low', { 'critique-low': { model: 'deepseek-v4-flash', effort: 'medium' } }, true)).toEqual({
      tier: 'low',
      overrides: { 'critique-low': { provider: 'deepseek', model: 'deepseek-v4-flash', effort: 'medium' } },
    });
    expect(resolveTierSubmission(CATALOG, 'low', { 'critique-low': { model: 'deepseek-v4-flash', effort: 'high' } }, true)).toHaveProperty(
      'error',
    );
  });

  it('해당 티어에 없는 에이전트를 거절한다', () => {
    expect(resolveTierSubmission(CATALOG, 'low', { 'research-medium': { model: 'claude-sonnet-5', effort: 'medium' } }, true)).toEqual({
      error: '이 티어에 없는 에이전트예요: research-medium',
    });
    expect(resolveTierSubmission(CATALOG, 'high', { editor: { model: 'claude-opus-5', effort: 'high' } }, true)).toHaveProperty('error');
  });

  it('미지 모델·무효 effort를 거절한다', () => {
    expect(resolveTierSubmission(CATALOG, 'high', { 'calibration-high': { model: 'gpt-6', effort: 'high' } }, true)).toHaveProperty(
      'error',
    );
    expect(resolveTierSubmission(CATALOG, 'high', { 'description-high': { model: 'claude-opus-5', effort: 'none' } }, true)).toHaveProperty(
      'error',
    );
    expect(resolveTierSubmission(CATALOG, 'low', { 'critique-low': { model: 'gemini-3.6-flash', effort: 'xhigh' } }, true)).toHaveProperty(
      'error',
    );
  });

  it('기본값과 같은 명시 선택은 no-op이다', () => {
    expect(resolveTierSubmission(CATALOG, 'high', { 'calibration-high': { model: 'claude-opus-5', effort: 'high' } }, true)).toEqual({
      tier: 'high',
      overrides: {},
    });
    expect(resolveTierSubmission(CATALOG, 'medium', { 'critique-medium': { model: 'claude-sonnet-5', effort: 'high' } }, true)).toEqual({
      tier: 'medium',
      overrides: {},
    });
  });

  it('유효 오버라이드를 수용한다', () => {
    expect(resolveTierSubmission(CATALOG, 'medium', { 'rephrase-medium': { model: 'gpt-5.6-luna', effort: 'low' } }, true)).toEqual({
      tier: 'medium',
      overrides: { 'rephrase-medium': { provider: 'openai', model: 'gpt-5.6-luna', effort: 'low' } },
    });
  });
});
