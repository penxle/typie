export type SectionKey = 'progress' | 'strengths' | 'verdicts' | 'elevations' | 'patterns' | 'priorities';

export type DetailCounts = {
  progress?: string | null;
  strengths: readonly unknown[];
  verdicts: readonly unknown[];
  elevations: readonly unknown[];
  patterns: readonly unknown[];
  priorities: readonly unknown[];
};

export const SECTION_TITLES: Record<SectionKey, string> = {
  progress: '지난 리뷰에서 나아진 점',
  strengths: '읽는 사람에게 잘 닿은 대목',
  verdicts: '관점마다 어디까지 왔는지',
  elevations: '한 걸음 더 가 볼 자리',
  patterns: '반복해서 나타나는 습관',
  priorities: '손보실 순서',
};

const CAPTIONS: Record<SectionKey, ((count: number) => string) | null> = {
  progress: null,
  strengths: (count) => `${count}곳 — 다음 원고에서도 믿고 쓰셔도 좋은 힘이에요`,
  verdicts: (count) => `${count}가지 — 이 작품에서 특히 중요하게 본 관점들이에요`,
  elevations: (count) => `${count}곳 — 고칠 곳이 아니라, 이미 잘 되고 있는 곳에서 해 볼 수 있는 제안이에요`,
  patterns: (count) => `${count}가지 — 하나를 고치면 여러 곳이 함께 풀려요`,
  priorities: (count) => `${count}가지 — 먼저 고치면 뒤가 쉬워지는 순서예요`,
};

export const visibleSections = (detail: DetailCounts): SectionKey[] => {
  const standing: [SectionKey, boolean][] = [
    ['progress', (detail.progress ?? '').trim().length > 0],
    ['strengths', detail.strengths.length > 0],
    ['verdicts', detail.verdicts.length > 0],
    ['elevations', detail.elevations.length > 0],
    ['patterns', detail.patterns.length > 0],
    ['priorities', detail.priorities.length > 0],
  ];

  return standing.filter(([, stands]) => stands).map(([key]) => key);
};

// 번호는 보이는 절끼리 잇달아 센다 — 빈 절이 빠져도 01부터 구멍 없이 흐른다.
export const sectionNumber = (sections: readonly SectionKey[], key: SectionKey): string =>
  String(sections.indexOf(key) + 1).padStart(2, '0');

export const sectionCaption = (key: SectionKey, count: number): string | null => CAPTIONS[key]?.(count) ?? null;
