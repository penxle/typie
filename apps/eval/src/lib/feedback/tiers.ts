// prism src/apps/feedback/tiers.ts(모델 허용 목록)·prompts.ts(에이전트 15종과 기본값)·workflows/(티어별 에이전트
// 구성)의 미러 — 검증 정본은 prism이다. prism 쪽 값이 바뀌면 이 파일을 함께 갱신한다. 티어 이름은 prism
// 워크플로 이름과 같은 문자열이라, 여기 목록이 곧 시작 가능한 워크플로 목록이다. 무오버라이드 리뷰의
// modelConfig는 "시작 시점에 이 상수가 알던 기본값"의 스냅샷이라, 동기화가 깨진 기간엔 기본값 표시가 실제와
// 어긋날 수 있다(오버라이드 항목은 저장=전송=실행이라 항상 정확).
export const TIER_NAMES = ['high', 'medium', 'low'] as const;
export type TierName = (typeof TIER_NAMES)[number];

export type AgentName =
  | 'research-high'
  | 'plan-high'
  | 'review-high'
  | 'critique-high'
  | 'proofread-high'
  | 'rephrase-high'
  | 'conclude-high'
  | 'research-medium'
  | 'critique-medium'
  | 'proofread-medium'
  | 'rephrase-medium'
  | 'conclude-medium'
  | 'critique-low'
  | 'proofread-low'
  | 'rephrase-low';

export const TIER_AGENTS: Record<TierName, readonly AgentName[]> = {
  high: ['research-high', 'plan-high', 'review-high', 'critique-high', 'proofread-high', 'rephrase-high', 'conclude-high'],
  medium: ['research-medium', 'critique-medium', 'proofread-medium', 'rephrase-medium', 'conclude-medium'],
  low: ['critique-low', 'proofread-low', 'rephrase-low'],
};

export const MODELS = {
  'claude-fable-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
  'claude-opus-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
  'claude-sonnet-5': { provider: 'anthropic', efforts: ['low', 'medium', 'high', 'xhigh', 'max'] },
  'gpt-5.6-sol': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
  'gpt-5.6-terra': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
  'gpt-5.6-luna': { provider: 'openai', efforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'] },
  'gemini-3.6-flash': { provider: 'gemini', efforts: ['minimal', 'low', 'medium', 'high'] },
  'gemini-3.5-flash-lite': { provider: 'gemini', efforts: ['minimal', 'low', 'medium', 'high'] },
} as const;
export type TierModel = keyof typeof MODELS;

export const AGENT_DEFAULTS: Record<AgentName, { model: TierModel; effort: string }> = {
  'research-high': { model: 'claude-opus-5', effort: 'xhigh' },
  'plan-high': { model: 'claude-opus-5', effort: 'xhigh' },
  'review-high': { model: 'gpt-5.6-sol', effort: 'xhigh' },
  'critique-high': { model: 'claude-opus-5', effort: 'xhigh' },
  'proofread-high': { model: 'claude-opus-5', effort: 'xhigh' },
  'rephrase-high': { model: 'claude-opus-5', effort: 'xhigh' },
  'conclude-high': { model: 'claude-opus-5', effort: 'xhigh' },
  'research-medium': { model: 'claude-sonnet-5', effort: 'medium' },
  'critique-medium': { model: 'claude-sonnet-5', effort: 'high' },
  'proofread-medium': { model: 'claude-sonnet-5', effort: 'high' },
  'rephrase-medium': { model: 'claude-sonnet-5', effort: 'medium' },
  'conclude-medium': { model: 'claude-sonnet-5', effort: 'medium' },
  'critique-low': { model: 'gemini-3.6-flash', effort: 'high' },
  'proofread-low': { model: 'gemini-3.6-flash', effort: 'high' },
  'rephrase-low': { model: 'gemini-3.6-flash', effort: 'high' },
};

export type TierOverrides = Partial<Record<AgentName, { model: TierModel; effort: string }>>;
export type ModelConfig = Partial<Record<AgentName, { model: string; effort: string; overridden: boolean }>>;

// 시작 시점의 전체 유효 구성 스냅샷 — 모달 표시의 데이터 원천. 모든 시작이 한 경로로 이걸 저장한다.
// 그 티어에 속한 에이전트만 담으므로 다른 티어의 오버라이드는 자연히 빠진다.
export const buildModelConfig = (tier: TierName, overrides: TierOverrides | undefined): ModelConfig =>
  Object.fromEntries(
    TIER_AGENTS[tier].map((agent) => {
      const override = overrides?.[agent];
      return [agent, override ? { ...override, overridden: true } : { ...AGENT_DEFAULTS[agent], overridden: false }];
    }),
  );

// 폼 제출 판정 — high 무오버라이드는 누구나 통과, 그 밖의 티어 선택이나 오버라이드 항목은 admin만. 기본값과
// 같은 명시 선택은 no-op으로 떨군다. 값 검증은 select UI와 무관하게 서버가 다시 한다(위조 제출 방어).
export const resolveTierSubmission = (
  tier: string,
  raw: Record<string, { model?: string; effort?: string }>,
  admin: boolean,
): { tier: TierName; overrides: TierOverrides } | { error: string } => {
  if (!(TIER_NAMES as readonly string[]).includes(tier)) return { error: `알 수 없는 티어예요: ${tier}` };
  const tierName = tier as TierName;
  const entries = Object.entries(raw);
  if (!admin && (tierName !== 'high' || entries.length > 0)) return { error: '티어 설정은 운영자만 쓸 수 있어요' };
  const agents = TIER_AGENTS[tierName] as readonly string[];
  const overrides: TierOverrides = {};
  for (const [agent, pair] of entries) {
    if (!agents.includes(agent)) return { error: `이 티어에 없는 에이전트예요: ${agent}` };
    const model = pair.model ?? '';
    if (!Object.hasOwn(MODELS, model)) return { error: `알 수 없는 모델이에요: ${model}` };
    const effort = pair.effort ?? '';
    if (!(MODELS[model as TierModel].efforts as readonly string[]).includes(effort)) {
      return { error: `${model}에서 쓸 수 없는 effort예요: ${effort}` };
    }
    const agentName = agent as AgentName;
    if (AGENT_DEFAULTS[agentName].model === model && AGENT_DEFAULTS[agentName].effort === effort) continue;
    overrides[agentName] = { model: model as TierModel, effort };
  }
  return { tier: tierName, overrides };
};
