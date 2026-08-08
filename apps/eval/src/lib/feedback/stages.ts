export type StageKey = 'research' | 'plan' | 'critique' | 'proofread' | 'rephrase' | 'conclude';

export const STAGES: { key: StageKey; label: string }[] = [
  { key: 'research', label: '원고 살펴보기' },
  { key: 'plan', label: '계획 세우기' },
  { key: 'critique', label: '짚을 곳 찾기' },
  { key: 'proofread', label: '문장 살피기' },
  { key: 'rephrase', label: '전할 말 고르기' },
  { key: 'conclude', label: '마무리 글 쓰기' },
];

const PREFIXES: [string, StageKey | null][] = [
  ['manuscript', null],
  ['research', 'research'],
  ['plan-review', 'plan'],
  ['plan-revise', 'plan'],
  ['plan', 'plan'],
  ['audit', 'plan'],
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

// 계획 왕복은 같은 스테이지 안에서 일어나 스테이지로는 드러나지 않는다 — 몇 번째 되짚기인지는 스텝 이름의 번호가 유일한 근거다.
export const nestedRound = (step: string | null): number | null => {
  const match = step === null ? null : /^plan-review-(\d+)/.exec(step);
  return match === null ? null : Number(match[1]);
};

export const TERMINAL_EVENTS = new Set<string>(['run.completed', 'run.failed', 'run.canceled']);
