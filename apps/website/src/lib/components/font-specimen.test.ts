import { describe, expect, it } from 'vitest';
import { buildFontSpecimenSearchParams, buildFontSpecimenUrl, familySpecimenFallbacks, weightSpecimenFallbacks } from './font-specimen';

describe('buildFontSpecimenSearchParams', () => {
  it('serializes repeated fallbacks in order', () => {
    expect(buildFontSpecimenSearchParams('보통', ['Regular', '400']).toString()).toBe(
      'text=%EB%B3%B4%ED%86%B5&fallbacks=Regular&fallbacks=400',
    );
  });
});

describe('buildFontSpecimenUrl', () => {
  it('builds the specimen endpoint url with repeated fallbacks', () => {
    expect(buildFontSpecimenUrl('https://api.typie.dev', 'font-123', '보통', ['Regular', '400'])).toBe(
      'https://api.typie.dev/font/font-123/specimen?text=%EB%B3%B4%ED%86%B5&fallbacks=Regular&fallbacks=400',
    );
  });
});

describe('familySpecimenFallbacks', () => {
  it('keeps only the family name when it differs from the display name', () => {
    expect(familySpecimenFallbacks('프리텐다드', 'Pretendard')).toEqual(['Pretendard']);
    expect(familySpecimenFallbacks('Pretendard', 'Pretendard')).toEqual([]);
  });
});

describe('weightSpecimenFallbacks', () => {
  it('keeps subfamily then numeric fallback', () => {
    expect(weightSpecimenFallbacks('보통', 'Regular', 400)).toEqual(['Regular', '400']);
    expect(weightSpecimenFallbacks('400', null, 400)).toEqual([]);
  });
});
