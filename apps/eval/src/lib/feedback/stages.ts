import type { TierName } from './tiers.ts';

// high는 판정 최후 파이프라인, medium·low는 기존 구성이다 — 두 집합의 합이 이 앱이 그릴 수 있는 단계다.
export type StageKey =
  | 'classify'
  | 'description'
  | 'interpretation'
  | 'rubric'
  | 'judgment'
  | 'stylistic'
  | 'delivery'
  | 'research'
  | 'critique'
  | 'proofread'
  | 'rephrase'
  | 'conclude';

export const STAGES: { key: StageKey; label: string }[] = [
  // 사전 분류 — 전 티어 공통 선두. 거부 판정이면 여기서 워크플로가 종결된다.
  { key: 'classify', label: '원고 가늠하기' },
  { key: 'description', label: '작품 읽기' },
  { key: 'interpretation', label: '작품 이해하기' },
  { key: 'rubric', label: '기준 세우기' },
  { key: 'judgment', label: '짚을 곳 찾기' },
  { key: 'stylistic', label: '문장 살피기' },
  { key: 'delivery', label: '전할 말 정리하기' },
  { key: 'research', label: '원고 살펴보기' },
  { key: 'critique', label: '짚을 곳 찾기' },
  { key: 'proofread', label: '문장 살피기' },
  { key: 'rephrase', label: '전할 말 고르기' },
  { key: 'conclude', label: '마무리 글 쓰기' },
];

// 티어마다 도는 단계가 다르다 — 그 티어에 에이전트가 없는 단계는 대기 카드로도 세우지 않는다.
export const TIER_STAGES: Record<TierName, StageKey[]> = {
  high: ['classify', 'description', 'interpretation', 'rubric', 'judgment', 'stylistic', 'delivery'],
  medium: ['classify', 'research', 'critique', 'proofread', 'rephrase', 'conclude'],
  low: ['classify', 'critique', 'proofread', 'rephrase'],
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
  ['research', 'research'],
  ['critique', 'critique'],
  ['proofread', 'proofread'],
  ['remarks', 'rephrase'],
  ['rephrase', 'rephrase'],
  ['tally', 'conclude'],
  ['conclude', 'conclude'],
];

export const stepStage = (stepName: string): StageKey | null => {
  for (const [prefix, key] of PREFIXES) {
    if (stepName === prefix || stepName.startsWith(`${prefix}-`)) return key;
  }
  return null;
};

// 에이전트 이름 → 스테이지. 어휘는 prism 에이전트 이름({stage}-{tier}, 계획 점검만 review-{tier})이고 재리뷰는
// -followup 접미가 붙는다(prism src/apps/feedback/followup.ts:26) — 접두 매칭이라 접미는 자연 흡수된다.
const AGENT_PREFIXES: [string, StageKey][] = [
  ['classify', 'classify'],
  ['description', 'description'],
  ['interpretation', 'interpretation'],
  ['rubric', 'rubric'],
  ['calibration', 'rubric'],
  ['judgment', 'judgment'],
  ['stylistic', 'stylistic'],
  ['delivery', 'delivery'],
  ['research', 'research'],
  ['critique', 'critique'],
  ['proofread', 'proofread'],
  ['rephrase', 'rephrase'],
  ['conclude', 'conclude'],
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
