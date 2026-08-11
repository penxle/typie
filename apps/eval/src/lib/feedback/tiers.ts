// 모델 허용 목록·에이전트 기본값·티어별 에이전트 구성의 정본은 prism이고, eval은 시작 시점마다 카탈로그
// 표면(GET /apps/feedback/catalog — fetchCatalog)을 걷어 쓴다. 정적 미러는 폐기됐다(오너 결정 2026-08-13) —
// 이 파일에 남는 것은 화면 어휘(티어 이름)와 카탈로그를 재료로 쓰는 조립·검증 헬퍼뿐이다.
// 티어 이름이 로컬 상수로 남는 이유: 단계 카드·라벨(stages.ts TIER_STAGES)이 티어를 화면 지식으로 알아야
// 해서, prism에 티어가 늘어도 화면 작업 없이는 노출할 수 없다 — 카탈로그에 없는 티어는 시작 검증이 거른다.
export const TIER_NAMES = ['high', 'medium', 'low'] as const;
export type TierName = (typeof TIER_NAMES)[number];

// 카탈로그 표면의 형태 — prism surface/entrypoint.ts AppCatalog의 사영이다.
export type AppCatalog = {
  models: Record<string, { provider: string; efforts: string[] }>;
  agents: Record<string, { provider: string; model: string; effort: string | null }>;
  workflows: Record<string, { agents: string[] }>;
};

// 오버라이드 와이어는 provider까지 명시하는 3장이다(prism workflows/shared.ts override와 한 짝) — eval이
// 카탈로그에서 provider를 채워 보내고, prism은 역추론 없이 3장 그대로 검증·소비한다.
export type TierOverrides = Partial<Record<string, { provider: string; model: string; effort: string }>>;

// 시작 시점 스냅샷 — 카탈로그 컷오버(2026-08-13)부터 provider가 실린다. 그 전 스냅샷에는 없으므로
// 소비자(비용 합성·재리뷰 승계)는 부재를 미상·카탈로그 폴백으로 다룬다.
export type ModelConfig = Partial<Record<string, { provider?: string; model: string; effort: string; overridden: boolean }>>;

// 시작 시점의 전체 유효 구성 스냅샷 — 모달 표시의 데이터 원천. 모든 시작이 한 경로로 이걸 저장한다.
// 행 목록과 순서는 카탈로그의 워크플로 구동 목록이 정한다(표시 순서 겸용).
export const buildModelConfig = (catalog: AppCatalog, tier: TierName, overrides: TierOverrides | undefined): ModelConfig =>
  Object.fromEntries(
    (catalog.workflows[tier]?.agents ?? []).map((agent) => {
      const override = overrides?.[agent];
      if (override) return [agent, { ...override, overridden: true }];
      const defaults = catalog.agents[agent];
      return [
        agent,
        { provider: defaults?.provider ?? '', model: defaults?.model ?? '', effort: defaults?.effort ?? '', overridden: false },
      ];
    }),
  );

// buildModelConfig의 역 — 재리뷰가 지난 회차의 오버라이드를 승계할 때 스냅샷에서 되돌린다. overridden 항목만
// 모으므로 스냅샷 시점의 기본값은 따라오지 않는다(기본값은 새 시작 시점의 카탈로그가 다시 정한다).
// 컷오버 전 스냅샷에는 provider가 없다 — 현재 카탈로그에서 채우고, 못 채우면(모델이 목록에서 빠짐) 빈
// provider로 두어 prism 시작 검증이 명시 반려한다. 부분 승계로 변인을 섞는 것보다 명시 실패가 낫다.
export const overridesFromConfig = (catalog: AppCatalog, config: ModelConfig | null): TierOverrides => {
  const overrides: TierOverrides = {};
  for (const [agent, entry] of Object.entries(config ?? {})) {
    if (!entry?.overridden) continue;
    overrides[agent] = {
      provider: entry.provider ?? catalog.models[entry.model]?.provider ?? '',
      model: entry.model,
      effort: entry.effort,
    };
  }
  return overrides;
};

// 폼 제출 판정 — high 무오버라이드는 누구나 통과, 그 밖의 티어 선택이나 오버라이드 항목은 admin만. 기본값과
// 같은 명시 선택은 no-op으로 떨군다. 값 검증은 select UI와 무관하게 서버가 카탈로그로 다시 하고(위조 제출
// 방어), prism이 시작 입력 검증으로 또 한 번 한다.
export const resolveTierSubmission = (
  catalog: AppCatalog,
  tier: string,
  raw: Record<string, { model?: string; effort?: string }>,
  admin: boolean,
): { tier: TierName; overrides: TierOverrides } | { error: string } => {
  if (!(TIER_NAMES as readonly string[]).includes(tier)) return { error: `알 수 없는 티어예요: ${tier}` };
  const tierName = tier as TierName;
  const entries = Object.entries(raw);
  if (!admin && (tierName !== 'high' || entries.length > 0)) return { error: '티어 설정은 운영자만 쓸 수 있어요' };
  const workflow = catalog.workflows[tierName];
  if (!workflow) return { error: `카탈로그에 없는 티어예요: ${tier}` };
  const overrides: TierOverrides = {};
  for (const [agent, pair] of entries) {
    if (!workflow.agents.includes(agent)) return { error: `이 티어에 없는 에이전트예요: ${agent}` };
    const model = pair.model ?? '';
    const listed = catalog.models[model];
    if (!listed) return { error: `알 수 없는 모델이에요: ${model}` };
    const effort = pair.effort ?? '';
    if (!listed.efforts.includes(effort)) return { error: `${model}에서 쓸 수 없는 effort예요: ${effort}` };
    const defaults = catalog.agents[agent];
    if (defaults && defaults.model === model && defaults.effort === effort) continue;
    overrides[agent] = { provider: listed.provider, model, effort };
  }
  return { tier: tierName, overrides };
};
