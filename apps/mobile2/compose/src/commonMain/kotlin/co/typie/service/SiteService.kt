package co.typie.service

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.storage.prefs

internal fun resolveStoredSiteId(
  userId: String,
  availableSiteIds: List<String>,
  siteIdsByUserId: Map<String, String>,
  legacySiteId: String,
): String {
  val storedSiteId = siteIdsByUserId[userId]
  val preferredSiteId = storedSiteId ?: legacySiteId.takeIf { it.isNotEmpty() }

  return preferredSiteId?.takeIf { it in availableSiteIds } ?: availableSiteIds.firstOrNull() ?: ""
}

object SiteService {
  private var currentUserId: String by prefs("current_user_id", "")
  private var siteIdsByUserId: Map<String, String> by
    prefs("site_ids_by_user_id", emptyMap<String, String>())
  private var legacySiteId: String by prefs("site_id", "")
  private var currentSiteId by mutableStateOf(loadCurrentSiteId())

  var siteId: String
    get() = currentSiteId
    set(value) {
      val userId = currentUserId
      if (userId.isEmpty()) return

      siteIdsByUserId = siteIdsByUserId + (userId to value)
      currentSiteId = value
    }

  fun bindUser(userId: String, availableSiteIds: List<String>) {
    currentUserId = userId

    val resolvedSiteId =
      resolveStoredSiteId(
        userId = userId,
        availableSiteIds = availableSiteIds,
        siteIdsByUserId = siteIdsByUserId,
        legacySiteId = legacySiteId,
      )

    if (resolvedSiteId.isNotEmpty()) {
      siteIdsByUserId = siteIdsByUserId + (userId to resolvedSiteId)
    }

    legacySiteId = ""
    currentSiteId = resolvedSiteId
  }

  fun clearCurrentUser() {
    currentUserId = ""
    currentSiteId = ""
  }

  private fun loadCurrentSiteId(): String {
    return currentUserId.takeIf { it.isNotEmpty() }?.let { siteIdsByUserId[it] }.orEmpty()
  }
}

//  private suspend fun fetchAuthenticatedUserContext(accessToken: String): AuthenticatedUser {
//    val response =
//      Http.post("${Konfig.API_URL}/graphql") {
//        header("Authorization", "Bearer $accessToken")
//        contentType(ContentType.Application.Json)
//        setBody("""{"query":"{ me { id sites { id } } }"}""")
//      }
//
//    val body = response.body<String>()
//    return AuthenticatedUser.fromBody(body)
//  }

// internal data class AuthenticatedUser(val userId: String, val siteIds: List<String>) {
//  companion object {
//    fun fromBody(body: String): AuthenticatedUser {
//      val json = Json.parseToJsonElement(body).jsonObject
//      val data = json["data"]?.jsonObject ?: error("Invalid access token")
//      val me =
//        data["me"]?.takeUnless { it.toString() == "null" }?.jsonObject
//          ?: error("Invalid access token")
//
//      val userId = me["id"]?.jsonPrimitive?.content ?: error("Invalid user id")
//      val siteIds =
//        me["sites"]
//          ?.jsonArray
//          ?.map { site ->
//            site.jsonObject["id"]?.jsonPrimitive?.content ?: error("Invalid site id")
//          }
//          .orEmpty()
//
//      return AuthenticatedUser(userId = userId, siteIds = siteIds)
//    }
//  }
// }
