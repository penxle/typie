import { foldCost } from './pricing.ts';
import type { AgentUsage } from './live.ts';
import type { PriceTable } from './pricing.ts';
import type { ModelConfig } from './tiers.ts';
import type { RunUsage, UsageFold } from './types.ts';

// 실행 중 화면의 fold 합성 — 턴 누적(에이전트별 4축)에 modelConfig 스냅샷의 provider·모델·effort를 입혀
// UsageFold 형태로 만든다. 구성 미상(스냅샷에 없는 에이전트·provider가 없는 컷오버 전 스냅샷)은 빈
// provider·model로 남겨 foldCost가 null을 내게 한다 — 미상을 떨어뜨리면 합계가 소리 없이 준다(부분합 금지,
// pricing.ts). 재리뷰 에이전트는 -followup 접미가 붙지만 modelConfig 키는 base 이름이다 — 벗겨 조회한다.
const FOLLOWUP_SUFFIX = '-followup';

export const synthesizeFolds = (usage: Record<string, AgentUsage>, modelConfig: ModelConfig | null): UsageFold[] =>
  Object.entries(usage).map(([agent, totals]) => {
    const base = agent.endsWith(FOLLOWUP_SUFFIX) ? agent.slice(0, -FOLLOWUP_SUFFIX.length) : agent;
    const entry = modelConfig?.[base];
    return {
      provider: entry?.provider ?? '',
      agent,
      model: entry?.model ?? '',
      effort: entry?.effort ?? null,
      turns: totals.turns,
      inputTokens: totals.inputTokens,
      outputTokens: totals.outputTokens,
      cacheReadTokens: totals.cacheReadTokens,
      cacheWriteTokens: totals.cacheWriteTokens,
      thinkingTokens: null,
    };
  });

// 부재(undefined)와 미상(null)의 병합 — 부재는 값으로 채우고, 미상은 한 번 섞이면 누적을 오염시킨다.
const addKrw = (prev: number | null | undefined, krw: number | null): number | null =>
  prev === undefined ? krw : prev === null || krw === null ? null : prev + krw;

// 정보 모달의 통합 표는 에이전트 축이다 — fold를 base 이름으로 접어(재리뷰 -followup 흡수) 구성 행에 비용을
// 나란히 세운다. 알려진 에이전트 밖의 fold는 기타로 접고, 단가를 모르는 fold(foldCost null)는 그 행과 합계를
// 함께 미상으로 만든다. etc 부재(undefined) = 기타 없음, 값 null = 미상 — 화면은 둘 다 —로 그리되 부재 행은
// 세우지 않는다.
export type AgentCostSummary = {
  agents: Partial<Record<string, number | null>>;
  etc?: number | null;
  total: number | null;
};

export const summarizeAgentCosts = (folds: UsageFold[], known: readonly string[], table: PriceTable | null): AgentCostSummary => {
  const summary: AgentCostSummary = { agents: {}, total: 0 };
  for (const entry of folds) {
    const base = entry.agent.endsWith(FOLLOWUP_SUFFIX) ? entry.agent.slice(0, -FOLLOWUP_SUFFIX.length) : entry.agent;
    const krw = foldCost(entry, table)?.krw ?? null;
    if (known.includes(base)) summary.agents[base] = addKrw(summary.agents[base], krw);
    else summary.etc = addKrw(summary.etc, krw);
    summary.total = addKrw(summary.total, krw);
  }
  return summary;
};

// 종결 usage의 하한 판정 — 사영이 settled:false를 complete:false로 보존한 기록(project.ts)은 아직 접히지 않은
// fold가 남았다는 뜻이라 합계가 하한이다. 기록 자체가 없으면 하한이 아니라 부재다.
export const usageLowerBound = (usage: RunUsage | null): boolean => usage !== null && !usage.complete;
