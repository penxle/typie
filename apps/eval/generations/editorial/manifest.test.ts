import { describe, expect, it } from 'vitest';
import { promptPhases, validatePromptSet } from '../../core/prompt-set.ts';
import { EDITORIAL_MANIFEST } from './manifest.ts';

describe('EDITORIAL_MANIFEST', () => {
  it('프롬프트 단계 일곱을 선언한다', () => {
    expect(promptPhases(EDITORIAL_MANIFEST).map((p) => p.key)).toEqual([
      'research',
      'plan',
      'planReview',
      'execute',
      'local',
      'compose',
      'composeReview',
    ]);
  });

  it('항목 종류 여섯을 선언한다', () => {
    expect(EDITORIAL_MANIFEST.itemKinds.map((k) => k.key)).toEqual([
      'characterization',
      'finding',
      'strength',
      'cleared',
      'pattern',
      'priority',
    ]);
  });

  it('layer facet으로 목록을 나눈다', () => {
    expect(EDITORIAL_MANIFEST.facets.find((f) => f.key === 'layer')?.groupBy).toBe(true);
  });

  it('빈 프롬프트 세트는 일곱 단계 모두를 위반으로 보고한다', () => {
    expect(validatePromptSet(EDITORIAL_MANIFEST, {})).toHaveLength(7);
  });
});
