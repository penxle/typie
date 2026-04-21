package co.typie.shell

data class MainDrawerSelection(val currentSiteId: String, val otherSiteIds: List<String>)

fun resolveMainDrawerSelection(
  selectedSiteId: String,
  availableSiteIds: List<String>,
): MainDrawerSelection {
  val currentSiteId =
    availableSiteIds.firstOrNull { it == selectedSiteId } ?: availableSiteIds.first()

  return MainDrawerSelection(
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
