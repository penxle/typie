package co.typie.shell

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class MainDrawerModelsTest {
  @Test
  fun `resolveMainDrawerSelection returns current site and others when selected site exists`() {
    assertEquals(
      MainDrawerSelection(currentSiteId = "site-b", otherSiteIds = listOf("site-a", "site-c")),
      resolveMainDrawerSelection(
        selectedSiteId = "site-b",
        availableSiteIds = listOf("site-a", "site-b", "site-c"),
      ),
    )
  }

  @Test
  fun `resolveMainDrawerSelection falls back to first site when selected site is missing`() {
    assertEquals(
      MainDrawerSelection(currentSiteId = "site-a", otherSiteIds = listOf("site-b")),
      resolveMainDrawerSelection(
        selectedSiteId = "site-z",
        availableSiteIds = listOf("site-a", "site-b"),
      ),
    )
  }

  @Test
  fun `resolvePendingCreatedSiteSelection returns id when present in available sites`() {
    assertEquals(
      "site-new",
      resolvePendingCreatedSiteSelection(
        pendingCreatedSiteId = "site-new",
        availableSiteIds = listOf("site-a", "site-new"),
      ),
    )
  }

  @Test
  fun `resolvePendingCreatedSiteSelection returns null when id missing`() {
    assertNull(
      resolvePendingCreatedSiteSelection(
        pendingCreatedSiteId = "site-new",
        availableSiteIds = listOf("site-a"),
      )
    )
  }

  @Test
  fun `resolvePendingCreatedSiteSelection returns null when id is null`() {
    assertNull(
      resolvePendingCreatedSiteSelection(
        pendingCreatedSiteId = null,
        availableSiteIds = listOf("site-a"),
      )
    )
  }
}
