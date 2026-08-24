import { describe, expect, it } from 'vitest';
import { detailOutline, GROUP_TITLES, SECTION_TITLES } from './detail-view.ts';
import type { DetailCounts } from './detail-view.ts';

const detail = (over: Partial<DetailCounts> = {}): DetailCounts => ({
  understanding: null,
  progress: null,
  strengths: [],
  verdicts: [],
  elevations: [],
  patterns: [],
  priorities: [],
  ...over,
});

const keysOf = (groups: ReturnType<typeof detailOutline>) => groups.map((group) => [group.key, group.sections.map((s) => s.key)]);

describe('detailOutline', () => {
  it('절을 세 덩어리로 묶고 총평 순서대로 세운다', () => {
    const groups = detailOutline(
      detail({
        understanding: '읽음',
        verdicts: [{}],
        progress: '나아짐',
        strengths: [{}],
        elevations: [{}],
        patterns: [{}],
        priorities: [{}],
      }),
    );

    expect(keysOf(groups)).toEqual([
      ['reading', ['understanding', 'verdicts', 'progress']],
      ['strong', ['strengths', 'elevations']],
      ['work', ['patterns', 'priorities']],
    ]);
  });

  it('빈 절은 서지 않는다', () => {
    const groups = detailOutline(detail({ understanding: '읽음', priorities: [{}] }));
    expect(keysOf(groups)).toEqual([
      ['reading', ['understanding']],
      ['work', ['priorities']],
    ]);
  });

  it('산문 절은 공백뿐이면 서지 않는다', () => {
    expect(detailOutline(detail({ understanding: ' '.repeat(3), progress: '\n' }))).toEqual([]);
  });

  it('절이 전부 빈 덩어리는 머리글째 빠진다', () => {
    const groups = detailOutline(detail({ patterns: [{}] }));
    expect(groups).toHaveLength(1);
    expect(groups[0].key).toBe('work');
  });

  it('번호는 보이는 절끼리 잇달아 세어 구멍이 없다', () => {
    const groups = detailOutline(detail({ understanding: '읽음', strengths: [{}], priorities: [{}] }));
    const numbers = groups.flatMap((group) => group.sections.map((section) => section.number));
    expect(numbers).toEqual(['01', '02', '03']);
  });

  it('앞 절이 빠져도 번호는 01부터 이어진다', () => {
    const groups = detailOutline(detail({ patterns: [{}], priorities: [{}] }));
    expect(groups.flatMap((group) => group.sections.map((section) => section.number))).toEqual(['01', '02']);
  });

  it('개수를 셀 수 없는 산문 절에는 캡션이 없다', () => {
    const groups = detailOutline(detail({ understanding: '읽음', progress: '나아짐', patterns: [{}, {}] }));
    const captions = new Map(groups.flatMap((group) => group.sections.map((section) => [section.key, section.caption])));

    expect(captions.get('understanding')).toBeNull();
    expect(captions.get('progress')).toBeNull();
    expect(captions.get('patterns')).not.toBeNull();
  });

  it('캡션은 항목 수를 싣는다', () => {
    const groups = detailOutline(detail({ strengths: [{}, {}, {}] }));
    expect(groups[0].sections[0].caption).toContain('3');
  });

  it('덩어리와 절의 이름이 빠짐없이 있다', () => {
    expect(Object.keys(GROUP_TITLES)).toHaveLength(3);
    expect(Object.keys(SECTION_TITLES)).toHaveLength(7);
  });
});
