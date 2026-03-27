package co.typie.ui.component

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class SpacePopoverModelsTest {
  @Test
  fun `resolveSpacePopoverSelection returns current site and others when selected site exists`() {
    assertEquals(
      SpacePopoverSelection(
        currentSiteId = "site-b",
        otherSiteIds = listOf("site-a", "site-c"),
      ),
      resolveSpacePopoverSelection(
        selectedSiteId = "site-b",
        availableSiteIds = listOf("site-a", "site-b", "site-c"),
      ),
    )
  }

  @Test
  fun `resolveSpacePopoverSelection falls back to first site when selected site is missing`() {
    assertEquals(
      SpacePopoverSelection(
        currentSiteId = "site-a",
        otherSiteIds = listOf("site-b"),
      ),
      resolveSpacePopoverSelection(
        selectedSiteId = "site-z",
        availableSiteIds = listOf("site-a", "site-b"),
      ),
    )
  }

  @Test
  fun `resolveSpacePopoverSelection returns null when there are no sites`() {
    assertNull(resolveSpacePopoverSelection(selectedSiteId = "site-z", availableSiteIds = emptyList()))
  }
}
