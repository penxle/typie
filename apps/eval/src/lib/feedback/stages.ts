import type { TierName } from './tiers.ts';

// 전 티어가 판정 최후 파이프라인의 변주다 — high가 원형, medium은 해석·기준표가 빠진 감산(정적 기준표),
// low는 상류 준비 없이 판정·문면·전달만 도는 감산이다. 구 medium 구성(research→conclude)은 prism 재설계
// (2026-08-17)로 철거됐고 이 앱도 그 어휘를 함께 지운다 — 구 구성으로 굳은 세션의 과정 화면은 단계 카드가
// 서지 않는다(high plan 컷오버와 같은 수용).
export type StageKey = 'classify' | 'description' | 'interpretation' | 'rubric' | 'judgment' | 'stylistic' | 'delivery';

export const STAGES: { key: StageKey; label: string }[] = [
  // 사전 분류 — 전 티어 공통 선두. 거부 판정이면 여기서 워크플로가 종결된다.
  { key: 'classify', label: '원고 가늠하기' },
  { key: 'description', label: '작품 읽기' },
  { key: 'interpretation', label: '작품 이해하기' },
  { key: 'rubric', label: '기준 세우기' },
  { key: 'judgment', label: '짚을 곳 찾기' },
  { key: 'stylistic', label: '문장 살피기' },
  { key: 'delivery', label: '전할 말 정리하기' },
];

// 티어마다 도는 단계가 다르다 — 그 티어에 에이전트가 없는 단계는 대기 카드로도 세우지 않는다.
export const TIER_STAGES: Record<TierName, StageKey[]> = {
  high: ['classify', 'description', 'interpretation', 'rubric', 'judgment', 'stylistic', 'delivery'],
  medium: ['classify', 'description', 'judgment', 'stylistic', 'delivery'],
  low: ['classify', 'judgment', 'stylistic', 'delivery'],
};

export const stagesFor = (tier: TierName): { key: StageKey; label: string }[] =>
  STAGES.filter((stage) => TIER_STAGES[tier].includes(stage.key));

const PREFIXES: [string, StageKey | null][] = [
  ['manuscript', null],
  ['meta', null],
  ['classify', 'classify'],
  ['description', 'description'],
  ['interpretation', 'interpretation'],
  // 검수 왕복은 기준 세우기 안에서 도는 중첩이다 — 조사 기록·수정 제출·검수 스텝이 모두 그 스테이지에 귀속된다.
  ['audit', 'rubric'],
  ['rubric-revise', 'rubric'],
  ['rubric', 'rubric'],
  ['calibration', 'rubric'],
  ['judgment', 'judgment'],
  ['stylistic', 'stylistic'],
  ['findings', 'delivery'],
  ['delivery', 'delivery'],
];

export const stepStage = (stepName: string): StageKey | null => {
  for (const [prefix, key] of PREFIXES) {
    if (stepName === prefix || stepName.startsWith(`${prefix}-`)) return key;
  }
  return null;
};

// 에이전트 이름 → 스테이지. 어휘는 prism 에이전트 이름({stage}-{tier})이고 재리뷰는 -followup 접미가 붙는다
// (prism src/apps/feedback/followup.ts) — 접두 매칭이라 접미는 자연 흡수된다.
const AGENT_PREFIXES: [string, StageKey][] = [
  ['classify', 'classify'],
  ['description', 'description'],
  ['interpretation', 'interpretation'],
  ['rubric', 'rubric'],
  ['calibration', 'rubric'],
  ['judgment', 'judgment'],
  ['stylistic', 'stylistic'],
  ['delivery', 'delivery'],
];

export const agentStage = (agentName: string): StageKey | null => {
  for (const [prefix, key] of AGENT_PREFIXES) {
    if (agentName === prefix || agentName.startsWith(`${prefix}-`)) return key;
  }
  return null;
};

// 검수 왕복은 같은 스테이지 안에서 일어나 스테이지로는 드러나지 않는다 — 몇 번째 되짚기인지는 스텝 이름의 번호가 유일한 근거다.
export const nestedRound = (step: string | null): number | null => {
  const match = step === null ? null : /^calibration-(\d+)/.exec(step);
  return match === null ? null : Number(match[1]);
};

export const TERMINAL_EVENTS = new Set<string>(['workflow.completed', 'workflow.failed', 'workflow.canceled']);
