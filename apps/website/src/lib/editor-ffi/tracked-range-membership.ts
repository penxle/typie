import type { Selection, StateField, TrackedRangeEndpoints } from '@typie/editor-ffi/browser';

type MembershipQuery = (selection: Selection) => TrackedRangeEndpoints[];

/** 문서 의미가 바뀔 때만 현재 selection의 Core membership을 다시 읽는다. */
export const semanticMembershipForStateChange = (
  fields: ReadonlySet<StateField>,
  selection: Selection | undefined,
  query: MembershipQuery,
): TrackedRangeEndpoints[] | undefined => {
  if (!fields.has('selection') && !fields.has('doc') && !fields.has('tracked_ranges')) return undefined;
  if (!selection) return undefined;
  return query(selection);
};

const featureMembers = (
  members: readonly TrackedRangeEndpoints[],
  allowedGroups: ReadonlySet<string>,
  ownedIds?: ReadonlySet<string>,
): TrackedRangeEndpoints[] =>
  members.filter((range) => allowedGroups.has(range.group) && (ownedIds === undefined || ownedIds.has(range.id)));

/** normal/active group 투영을 제외한 feature membership의 결정적 identity다. */
export const trackedRangeMembershipIds = (
  members: readonly TrackedRangeEndpoints[],
  allowedGroups: ReadonlySet<string>,
  ownedIds?: ReadonlySet<string>,
): string[] => featureMembers(members, allowedGroups, ownedIds).map((range) => range.id);

/** Core fallback 순서를 유지하되 현재 feature의 eligible active ID를 먼저 고른다. */
export const selectTrackedRangeMember = (
  members: readonly TrackedRangeEndpoints[],
  allowedGroups: ReadonlySet<string>,
  activeId: string | null,
  ownedIds?: ReadonlySet<string>,
): TrackedRangeEndpoints | undefined => {
  const candidates = featureMembers(members, allowedGroups, ownedIds);
  return (activeId === null ? undefined : candidates.find((range) => range.id === activeId)) ?? candidates[0];
};
