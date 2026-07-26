import { describe, expect, it } from 'vitest';
import { buildBackgroundQuery, MAX_QUERY_NOUNS, renderSearchHits } from './search.ts';

describe('buildBackgroundQuery', () => {
  it('원작명이 있으면 그것을 주어로 삼는다', () => {
    const q = buildBackgroundQuery({ derivativeSource: '천관사복', properNouns: ['사무도', '사청현', '풍사선'] });
    expect(q).toBe('천관사복 등장인물 관계 설정 줄거리 사무도 사청현 풍사선');
  });

  it('원작명이 없으면 고유명사로 되짚는다', () => {
    const q = buildBackgroundQuery({ derivativeSource: null, properNouns: ['시마', '이부키'] });
    expect(q).toBe('시마 이부키 등장인물 원작 어느 작품');
  });

  it("'원작 불명'은 원작명이 없는 것으로 본다", () => {
    const q = buildBackgroundQuery({ derivativeSource: '원작 불명', properNouns: ['시마', '이부키'] });
    expect(q).toBe('시마 이부키 등장인물 원작 어느 작품');
  });

  // 이름 하나로 검색하면 엉뚱한 원작을 물어온다. 잘못된 배경은 배경이 없는 것보다 나쁘다.
  it('단서가 너무 적으면 검색하지 않는다', () => {
    expect(buildBackgroundQuery({ derivativeSource: null, properNouns: ['시마'] })).toBeNull();
    expect(buildBackgroundQuery({ derivativeSource: null, properNouns: [] })).toBeNull();
  });

  it('고유명사가 많아도 상한까지만 쓴다', () => {
    const nouns = Array.from({ length: 20 }, (_, i) => `이름${i}`);
    const q = buildBackgroundQuery({ derivativeSource: null, properNouns: nouns });
    expect(q?.split(' ').filter((w) => w.startsWith('이름'))).toHaveLength(MAX_QUERY_NOUNS);
  });

  it('빈 문자열·공백 고유명사는 버린다', () => {
    const q = buildBackgroundQuery({ derivativeSource: null, properNouns: ['  ', '시마', '', ' 이부키 '] });
    expect(q).toBe('시마 이부키 등장인물 원작 어느 작품');
  });
});

describe('renderSearchHits', () => {
  it('번호·제목·출처를 붙여 늘어놓는다', () => {
    const out = renderSearchHits([
      { title: '가', url: 'https://a', text: '본문가' },
      { title: '나', url: 'https://b', text: '본문나' },
    ]);
    expect(out).toContain('[1] 가');
    expect(out).toContain('https://b');
    expect(out.split('---')).toHaveLength(2);
  });

  it('결과가 없으면 빈 문자열', () => {
    expect(renderSearchHits([])).toBe('');
  });
});
