import type { PrismReviewTierName } from '@typie/prism';

export type StageKey = 'classify' | 'description' | 'interpretation' | 'rubric' | 'judgment' | 'stylistic' | 'delivery';

export const STAGES: { key: StageKey; label: string }[] = [
  { key: 'classify', label: '원고 가늠하기' },
  { key: 'description', label: '작품 읽기' },
  { key: 'interpretation', label: '작품 이해하기' },
  { key: 'rubric', label: '기준 세우기' },
  { key: 'judgment', label: '짚을 곳 찾기' },
  { key: 'stylistic', label: '문장 살피기' },
  { key: 'delivery', label: '전할 말 정리하기' },
];

export const TIER_STAGES: Record<PrismReviewTierName, StageKey[]> = {
  high: ['classify', 'description', 'interpretation', 'rubric', 'judgment', 'stylistic', 'delivery'],
  medium: ['classify', 'description', 'judgment', 'stylistic', 'delivery'],
  low: ['classify', 'judgment', 'stylistic', 'delivery'],
};

export const stagesFor = (tier: PrismReviewTierName): { key: StageKey; label: string }[] =>
  STAGES.filter((stage) => TIER_STAGES[tier].includes(stage.key));

const STEP_PREFIXES: [string, StageKey][] = [
  ['classify', 'classify'],
  ['description', 'description'],
  ['interpretation', 'interpretation'],
  ['audit', 'rubric'],
  ['calibration', 'rubric'],
  ['rubric', 'rubric'],
  ['judgment', 'judgment'],
  ['stylistic', 'stylistic'],
  ['delivery', 'delivery'],
];

const EXCLUDED = new Set(['judgment-targets', 'stylistic-targets']);

export const stepStage = (step: string): StageKey | null => {
  if (EXCLUDED.has(step)) return null;
  for (const [prefix, key] of STEP_PREFIXES) {
    if (step === prefix || step.startsWith(`${prefix}-`)) return key;
  }
  return null;
};

export const stepRound = (step: string): number | null => {
  const match = /^(?:audit|calibration)-(\d+)(?:-\d+)?$/.exec(step);
  return match === null ? null : Number(match[1]);
};
