package co.typie.service

import kotlin.test.Test
import kotlin.test.assertEquals

class SiteServiceModelsTest {
  @Test
  fun `resolveStoredSiteId prefers per-user saved site`() {
    assertEquals(
      "site-2",
      resolveStoredSiteId(
        userId = "user-1",
        availableSiteIds = listOf("site-1", "site-2"),
        siteIdsByUserId = mapOf("user-1" to "site-2"),
        legacySiteId = "site-1",
      ),
    )
  }

  @Test
  fun `resolveStoredSiteId falls back to legacy site for first migration`() {
    assertEquals(
      "site-2",
      resolveStoredSiteId(
        userId = "user-1",
        availableSiteIds = listOf("site-1", "site-2"),
        siteIdsByUserId = emptyMap(),
        legacySiteId = "site-2",
      ),
    )
  }

  @Test
  fun `resolveStoredSiteId falls back to first available site when saved sites are invalid`() {
    assertEquals(
      "site-1",
      resolveStoredSiteId(
        userId = "user-1",
        availableSiteIds = listOf("site-1", "site-2"),
        siteIdsByUserId = mapOf("user-1" to "site-9"),
        legacySiteId = "site-8",
      ),
    )
  }
}
