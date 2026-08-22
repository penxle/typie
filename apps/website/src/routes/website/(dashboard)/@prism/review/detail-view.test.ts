import { describe, expect, it } from 'vitest';
import { SECTION_TITLES, sectionCaption, sectionNumber, visibleSections } from './detail-view.ts';
import type { DetailCounts } from './detail-view.ts';

const detail = (over: Partial<DetailCounts> = {}): DetailCounts => ({
  progress: null,
  strengths: [],
  verdicts: [],
  elevations: [],
  patterns: [],
  priorities: [],
  ...over,
});

describe('visibleSections', () => {
  it('비어 있지 않은 절만 총평 순서대로 세운다', () => {
    expect(visibleSections(detail({ patterns: [{}], strengths: [{}] }))).toEqual(['strengths', 'patterns']);
  });

  it('진전 서술은 문면이 있을 때만 선다', () => {
    expect(visibleSections(detail({ progress: '나아진 점' }))).toEqual(['progress']);
    expect(visibleSections(detail({ progress: ' '.repeat(3) }))).toEqual([]);
  });

  it('전부 비면 아무 절도 서지 않는다', () => {
    expect(visibleSections(detail())).toEqual([]);
  });
});

describe('sectionNumber', () => {
  it('보이는 절끼리 01부터 구멍 없이 센다', () => {
    const sections = visibleSections(detail({ verdicts: [{}], priorities: [{}] }));
    expect(sectionNumber(sections, 'verdicts')).toBe('01');
    expect(sectionNumber(sections, 'priorities')).toBe('02');
  });
});

describe('sectionCaption', () => {
  it('개수를 실은 안내를 낸다 — 진전 서술에는 안내가 없다', () => {
    expect(sectionCaption('strengths', 2)).toBe('2곳 — 다음 원고에서도 믿고 쓰셔도 좋은 힘이에요');
    expect(sectionCaption('priorities', 3)).toBe('3가지 — 먼저 고치면 뒤가 쉬워지는 순서예요');
    expect(sectionCaption('progress', 1)).toBeNull();
  });

  it('절 제목은 여섯 개가 모두 있다', () => {
    expect(Object.keys(SECTION_TITLES)).toHaveLength(6);
  });
});
