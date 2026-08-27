import { TIER_OPTIONS } from './tiers.ts';
import type { PrismReviewTierName } from '@typie/prism';

export type LineageOption = { id: string; tier: PrismReviewTierName; latestOrdinal: number; locked: boolean };

// 기본값 = 여백에 표시 중인 회차의 계보, 없으면 가장 최근 계보(목록 첫 항목). 진행 중 계보는 고를 수 없다
export const pickDefaultLineage = (lineages: readonly LineageOption[], shownLineageId: string | null): string | null => {
  const shown = lineages.find((lineage) => lineage.id === shownLineageId && !lineage.locked);
  return (shown ?? lineages.find((lineage) => !lineage.locked))?.id ?? null;
};

export const lineageRowLabel = (lineage: Pick<LineageOption, 'tier' | 'latestOrdinal'>): string =>
  `${lineage.latestOrdinal}회차에 이어서 · ${TIER_OPTIONS.find((option) => option.tier === lineage.tier)?.label ?? '리뷰'}`;
