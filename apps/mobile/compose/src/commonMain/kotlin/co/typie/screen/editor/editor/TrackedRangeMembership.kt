package co.typie.screen.editor.editor

import co.typie.editor.ffi.TrackedRangeEndpoints

private fun List<TrackedRangeEndpoints>.featureCandidates(
  allowedGroups: Set<String>,
  ownedIds: Set<String>?,
): List<TrackedRangeEndpoints> = filter { range ->
  range.group in allowedGroups && (ownedIds == null || range.id in ownedIds)
}

internal fun List<TrackedRangeEndpoints>.trackedRangeMembershipIds(
  allowedGroups: Set<String>,
  ownedIds: Set<String>? = null,
): List<String> = featureCandidates(allowedGroups, ownedIds).map { it.id }

/**
 * Core가 정한 membership fallback 순서를 보존하면서 feature가 사용할 한 범위를 고른다.
 *
 * 명시적인 active ID도 현재 cursor의 후보이면서 허용된 group/소유 ID에 속할 때만 우선한다.
 */
internal fun List<TrackedRangeEndpoints>.selectTrackedRangeMember(
  allowedGroups: Set<String>,
  activeId: String?,
  ownedIds: Set<String>? = null,
): TrackedRangeEndpoints? {
  val candidates = featureCandidates(allowedGroups, ownedIds)
  return activeId?.let { id -> candidates.firstOrNull { it.id == id } } ?: candidates.firstOrNull()
}
