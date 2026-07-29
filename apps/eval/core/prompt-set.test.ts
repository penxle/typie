import { describe, expect, it } from 'vitest';
import { promptPhases, resolvePrompts, validatePromptSet } from './prompt-set.ts';
import type { GenerationManifest } from './contracts.ts';

const manifest: GenerationManifest = {
  id: 'sample',
  label: '표본',
  status: 'active',
  phases: [
    { key: 'alpha', label: '알파' },
    { key: 'beta', label: '베타' },
    { key: 'gamma', label: '감마', prompt: false },
  ],
  itemKinds: [{ key: 'finding', label: '지적' }],
  facets: [],
  evaluations: [],
  artifacts: null,
};

const prompt = { system: 's', model: 'anthropic/claude-opus-5', effort: null };

describe('promptPhases', () => {
  it('prompt:false 단계를 제외한다', () => {
    expect(promptPhases(manifest).map((p) => p.key)).toEqual(['alpha', 'beta']);
  });
});

describe('validatePromptSet', () => {
  it('전부 채워지면 위반이 없다', () => {
    expect(validatePromptSet(manifest, { alpha: prompt, beta: prompt })).toEqual([]);
  });

  it('누락된 단계를 보고한다', () => {
    expect(validatePromptSet(manifest, { alpha: prompt })).toEqual(['beta 단계의 프롬프트가 없습니다']);
  });

  it('매니페스트에 없는 키를 보고한다', () => {
    expect(validatePromptSet(manifest, { alpha: prompt, beta: prompt, delta: prompt })).toEqual(['delta는 이 세대에 없는 단계입니다']);
  });

  it('prompt:false 단계에 프롬프트가 들어오면 보고한다', () => {
    expect(validatePromptSet(manifest, { alpha: prompt, beta: prompt, gamma: prompt })).toEqual(['gamma는 이 세대에 없는 단계입니다']);
  });

  it('model이 빠진 프롬프트를 보고한다', () => {
    expect(validatePromptSet(manifest, { alpha: prompt, beta: { system: 's' } })).toEqual(['beta 단계에 model이 없습니다']);
  });

  it('system이 빠진 프롬프트를 보고한다', () => {
    expect(validatePromptSet(manifest, { alpha: prompt, beta: { model: 'm' } })).toEqual(['beta 단계에 system이 없습니다']);
  });
});

describe('resolvePrompts', () => {
  it('위반이 있으면 던진다', () => {
    expect(() => resolvePrompts(manifest, { alpha: prompt })).toThrow('beta 단계의 프롬프트가 없습니다');
  });

  it('통과하면 좁혀진 맵을 돌려준다', () => {
    const resolved = resolvePrompts(manifest, { alpha: prompt, beta: prompt });
    expect(resolved.alpha.model).toBe('anthropic/claude-opus-5');
    expect(Object.keys(resolved)).toEqual(['alpha', 'beta']);
  });
});
