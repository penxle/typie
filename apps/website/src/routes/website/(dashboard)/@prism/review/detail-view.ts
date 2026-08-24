export type SectionKey = 'understanding' | 'verdicts' | 'progress' | 'strengths' | 'elevations' | 'patterns' | 'priorities';
export type GroupKey = 'reading' | 'strong' | 'work';

export type DetailCounts = {
  understanding?: string | null;
  progress?: string | null;
  strengths: readonly unknown[];
  verdicts: readonly unknown[];
  elevations: readonly unknown[];
  patterns: readonly unknown[];
  priorities: readonly unknown[];
};

export type Section = { key: SectionKey; number: string; caption: string | null };
export type Group = { key: GroupKey; sections: Section[] };

export const GROUP_TITLES: Record<GroupKey, string> = {
  reading: '작품 읽기',
  strong: '잘된 곳',
  work: '손볼 곳',
};

export const SECTION_TITLES: Record<SectionKey, string> = {
  understanding: '이렇게 읽었어요',
  verdicts: '지금 서 있는 자리',
  progress: '나아진 것',
  strengths: '잘 닿은 대목',
  elevations: '한 걸음 더',
  patterns: '반복되는 습관',
  priorities: '손보실 순서',
};

// 목차와 본문이 같은 순서로 서는 근거는 이 배열 하나다
const ORDER: readonly (readonly [GroupKey, SectionKey])[] = [
  ['reading', 'understanding'],
  ['reading', 'verdicts'],
  ['reading', 'progress'],
  ['strong', 'strengths'],
  ['strong', 'elevations'],
  ['work', 'patterns'],
  ['work', 'priorities'],
];

// 산문 절은 셀 항목이 없어 캡션이 없다
const CAPTIONS: Record<SectionKey, ((count: number) => string) | null> = {
  understanding: null,
  progress: null,
  verdicts: (count) => `관점 ${count}가지`,
  strengths: (count) => `${count}곳`,
  elevations: (count) => `${count}곳`,
  patterns: (count) => `${count}가지`,
  priorities: (count) => `${count}가지`,
};

const filled = (text: string | null | undefined) => ((text ?? '').trim().length > 0 ? 1 : 0);

const sizes = (detail: DetailCounts): Record<SectionKey, number> => ({
  understanding: filled(detail.understanding),
  progress: filled(detail.progress),
  verdicts: detail.verdicts.length,
  strengths: detail.strengths.length,
  elevations: detail.elevations.length,
  patterns: detail.patterns.length,
  priorities: detail.priorities.length,
});

export const detailOutline = (detail: DetailCounts): Group[] => {
  const size = sizes(detail);
  const groups = new Map<GroupKey, Section[]>();
  let seen = 0;

  for (const [group, key] of ORDER) {
    const count = size[key];
    if (count === 0) continue;

    seen += 1;
    const sections = groups.get(group) ?? [];
    sections.push({ key, number: String(seen).padStart(2, '0'), caption: CAPTIONS[key]?.(count) ?? null });
    groups.set(group, sections);
  }

  return [...groups].map(([key, sections]) => ({ key, sections }));
};
