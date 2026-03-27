package co.typie.ui.component

data class SpacePopoverSelection(
  val currentSiteId: String,
  val otherSiteIds: List<String>,
)

fun resolveSpacePopoverSelection(
  selectedSiteId: String,
  availableSiteIds: List<String>,
): SpacePopoverSelection? {
  val currentSiteId = availableSiteIds.firstOrNull { it == selectedSiteId }
    ?: availableSiteIds.firstOrNull()
    ?: return null

  return SpacePopoverSelection(
    currentSiteId = currentSiteId,
    otherSiteIds = availableSiteIds.filter { it != currentSiteId },
  )
}
