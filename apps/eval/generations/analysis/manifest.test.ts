import { describe, expect, it } from 'vitest';
import { ANALYSIS_MANIFEST } from './manifest.ts';

describe('ANALYSIS_MANIFEST', () => {
  it('동결 상태다', () => {
    expect(ANALYSIS_MANIFEST.status).toBe('frozen');
  });

  it('구 실행이 남긴 단계 이름을 전부 담는다', () => {
    const keys = new Set(ANALYSIS_MANIFEST.phases.map((p) => p.key));
    for (const stage of [
      'survey',
      'background',
      'genre',
      'plan',
      'planReview',
      'review',
      'dedupe',
      'verify',
      'compose',
      'composeReview',
      'selfcheck',
    ]) {
      expect(keys.has(stage)).toBe(true);
    }
  });

  it('cleared와 layer가 없다', () => {
    expect(ANALYSIS_MANIFEST.itemKinds.map((k) => k.key)).not.toContain('cleared');
    expect(ANALYSIS_MANIFEST.facets.map((f) => f.key)).not.toContain('layer');
  });
});
