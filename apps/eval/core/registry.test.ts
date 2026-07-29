import { describe, expect, it } from 'vitest';
import { evaluationById, generationById, GENERATIONS, qualifiedEvaluationId } from './registry.ts';

describe('registry', () => {
  it('두 세대를 담는다', () => {
    expect(GENERATIONS.map((g) => g.id).toSorted((a, b) => a.localeCompare(b))).toEqual(['analysis', 'editorial']);
  });

  it('없는 세대는 null', () => {
    expect(generationById('nope')).toBeNull();
  });

  it('세대 스코프 평가 id를 만든다', () => {
    expect(qualifiedEvaluationId('editorial', 'triaxial')).toBe('editorial/triaxial');
  });

  it('세대 스코프 id로 평가를 찾는다', () => {
    const found = evaluationById('editorial/triaxial');
    expect(found?.generation.id).toBe('editorial');
    expect(found?.evaluation.id).toBe('triaxial');
  });

  it('없는 평가와 잘못된 형식은 null', () => {
    expect(evaluationById('editorial/nope')).toBeNull();
    expect(evaluationById('malformed')).toBeNull();
  });
});
