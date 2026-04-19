package co.typie.ui.component

data class SpacePopoverSelection(val currentSiteId: String, val otherSiteIds: List<String>)

fun resolveSpacePopoverSelection(
  selectedSiteId: String,
  availableSiteIds: List<String>,
): SpacePopoverSelection {
  val currentSiteId =
    availableSiteIds.firstOrNull { it == selectedSiteId } ?: availableSiteIds.first()

  return SpacePopoverSelection(
    currentSiteId = currentSiteId,
    otherSiteIds = availableSiteIds.filter { it != currentSiteId },
  )
}

fun resolvePendingCreatedSiteSelection(
  pendingCreatedSiteId: String?,
  availableSiteIds: List<String>,
): String? {
  return pendingCreatedSiteId?.takeIf { it in availableSiteIds }
}
