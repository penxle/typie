import { PRISM_REVIEW_TIERS } from '@typie/prism';
import type { PrismReviewTierName } from '@typie/prism';

export type StageKey = 'classify' | 'description' | 'interpretation' | 'rubric' | 'judgment' | 'stylistic' | 'delivery';

export const STAGES: { key: StageKey; label: string; description: string }[] = [
  { key: 'classify', label: '원고 확인하기', description: '리뷰할 수 있는 원고인지 파악해요' },
  { key: 'description', label: '원고 읽기', description: '원고의 흐름과 서술 방식, 상정 독자를 정리해요' },
  { key: 'interpretation', label: '원고 이해하기', description: '작품이 하려는 일과 잘 실행된 자리를 적어요' },
  { key: 'rubric', label: '기준 세우기', description: '이 작품에 맞는 기준을 세우고, 자체 검수를 통해 조정해요' },
  { key: 'judgment', label: '내용 살피기', description: '기준에 걸리는 자리마다 피드백을 남겨요' },
  { key: 'stylistic', label: '문장 살피기', description: '문장과 표현에 걸리는 자리마다 피드백을 남겨요' },
  { key: 'delivery', label: '결과 정리하기', description: '피드백을 모아 정리하고 읽기 좋게 다듬어요' },
];

export const TIER_STAGES: Record<PrismReviewTierName, StageKey[]> = {
  high: ['classify', 'description', 'interpretation', 'rubric', 'judgment', 'stylistic', 'delivery'],
  medium: ['classify', 'description', 'judgment', 'stylistic', 'delivery'],
  low: ['classify', 'judgment', 'stylistic', 'delivery'],
};

export const stagesFor = (tier: PrismReviewTierName): { key: StageKey; label: string }[] =>
  STAGES.filter((stage) => TIER_STAGES[tier].includes(stage.key));

// 확인 카드가 지나치는 단계에 다는 「일반 검토부터」 태그의 출처 — 손으로 적으면 파이프라인이 바뀔 때
// 문구만 남는다. PRISM_REVIEW_TIERS가 낮은 티어부터라서 첫 적중이 곧 그 단계가 처음 열리는 티어다.
export const stageIntroducedIn = (key: StageKey): PrismReviewTierName =>
  PRISM_REVIEW_TIERS.find((tier) => TIER_STAGES[tier].includes(key)) ?? 'high';

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
