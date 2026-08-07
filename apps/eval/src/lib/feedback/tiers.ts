// prism src/apps/feedback/tiers.ts(허용 목록)·prompts.ts(에이전트 기본값)의 미러 — 검증 정본은 prism이다.
// prism 쪽 값이 바뀌면 이 파일을 함께 갱신한다. 무오버라이드 리뷰의 modelConfig는 "시작 시점에 이 상수가
// 알던 기본값"의 스냅샷이라, 동기화가 깨진 기간엔 기본값 표시가 실제와 어긋날 수 있다(오버라이드 항목은
// 저장=전송=실행이라 항상 정확).
export const AGENTS = ['research', 'plan', 'review', 'critique', 'proofread', 'rephrase', 'conclude'] as const;
export type AgentName = (typeof AGENTS)[number];

export const MODELS = {
  'claude-fable-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
  'claude-opus-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
  'claude-sonnet-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
  'gpt-5.6-sol': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
  'gpt-5.6-terra': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
  'gpt-5.6-luna': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
} as const;
export type TierModel = keyof typeof MODELS;

export const AGENT_DEFAULTS: Record<AgentName, { model: TierModel; effort: string }> = {
  research: { model: 'claude-opus-5', effort: 'xhigh' },
  plan: { model: 'claude-opus-5', effort: 'medium' },
  review: { model: 'gpt-5.6-sol', effort: 'high' },
  critique: { model: 'claude-opus-5', effort: 'xhigh' },
  proofread: { model: 'claude-opus-5', effort: 'xhigh' },
  rephrase: { model: 'claude-opus-5', effort: 'medium' },
  conclude: { model: 'claude-opus-5', effort: 'medium' },
};

export type TierOverrides = Partial<Record<AgentName, { model: TierModel; effort: string }>>;
export type ModelConfig = Record<AgentName, { model: string; effort: string; overridden: boolean }>;

// 시작 시점의 전체 유효 구성 스냅샷 — 모달 표시의 데이터 원천. 모든 시작이 한 경로로 이걸 저장한다.
export const buildModelConfig = (overrides: TierOverrides | undefined): ModelConfig =>
  Object.fromEntries(
    AGENTS.map((agent) => {
      const override = overrides?.[agent];
      return [agent, override ? { ...override, overridden: true } : { ...AGENT_DEFAULTS[agent], overridden: false }];
    }),
  ) as ModelConfig;

// 폼 제출 판정 — 빈 제출은 누구나 통과(오버라이드 없음), 티어 필드가 있으면 admin만. 기본값과 같은
// 명시 선택은 no-op으로 떨군다. 값 검증은 select UI와 무관하게 서버가 다시 한다(위조 제출 방어).
export const resolveTierSubmission = (
  raw: Record<string, { model?: string; effort?: string }>,
  admin: boolean,
): { overrides: TierOverrides } | { error: string } => {
  const entries = Object.entries(raw);
  if (entries.length === 0) return { overrides: {} };
  if (!admin) return { error: '티어 설정은 운영자만 쓸 수 있어요' };
  const overrides: TierOverrides = {};
  for (const [agent, pair] of entries) {
    if (!(AGENTS as readonly string[]).includes(agent)) return { error: `알 수 없는 에이전트예요: ${agent}` };
    const model = pair.model ?? '';
    if (!Object.hasOwn(MODELS, model)) return { error: `알 수 없는 모델이에요: ${model}` };
    const effort = pair.effort ?? '';
    if (!(MODELS[model as TierModel].efforts as readonly string[]).includes(effort)) {
      return { error: `${model}에서 쓸 수 없는 effort예요: ${effort}` };
    }
    const name = agent as AgentName;
    if (AGENT_DEFAULTS[name].model === model && AGENT_DEFAULTS[name].effort === effort) continue;
    overrides[name] = { model: model as TierModel, effort };
  }
  return { overrides };
};
