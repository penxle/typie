package co.typie.service

import co.typie.storage.Prefs
import org.koin.core.annotation.Single

internal fun resolveStoredSiteId(
  userId: String,
  availableSiteIds: List<String>,
  siteIdsByUserId: Map<String, String>,
  legacySiteId: String,
): String {
  val storedSiteId = siteIdsByUserId[userId]
  val preferredSiteId = storedSiteId ?: legacySiteId.takeIf { it.isNotEmpty() }

  return preferredSiteId?.takeIf { it in availableSiteIds }
    ?: availableSiteIds.firstOrNull()
    ?: ""
}

@Single
class SiteService(prefs: Prefs) {
  private var currentUserId: String by prefs("current_user_id", "")
  private var siteIdsByUserId: Map<String, String> by prefs("site_ids_by_user_id", emptyMap<String, String>())
  private var legacySiteId: String by prefs("site_id", "")

  var siteId: String
    get() = currentUserId.takeIf { it.isNotEmpty() }?.let { siteIdsByUserId[it] }.orEmpty()
    set(value) {
      val userId = currentUserId
      if (userId.isEmpty()) return

      siteIdsByUserId = siteIdsByUserId + (userId to value)
    }

  fun bindUser(userId: String, availableSiteIds: List<String>) {
    currentUserId = userId

    val resolvedSiteId = resolveStoredSiteId(
      userId = userId,
      availableSiteIds = availableSiteIds,
      siteIdsByUserId = siteIdsByUserId,
      legacySiteId = legacySiteId,
    )

    if (resolvedSiteId.isNotEmpty()) {
      siteIdsByUserId = siteIdsByUserId + (userId to resolvedSiteId)
    }

    legacySiteId = ""
  }

  fun clearCurrentUser() {
    currentUserId = ""
  }
}
