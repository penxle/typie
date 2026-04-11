package co.typie.bootstrap

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class BootstrapSiteValidationTest {
  @Test
  fun `returns current siteId when it exists in available sites`() {
    assertEquals(
      "site-2",
      resolvedSiteId(currentSiteId = "site-2", availableSiteIds = listOf("site-1", "site-2")),
    )
  }

  @Test
  fun `returns first available site when current siteId is not in list`() {
    assertEquals(
      "site-1",
      resolvedSiteId(currentSiteId = "site-9", availableSiteIds = listOf("site-1", "site-2")),
    )
  }

  @Test
  fun `returns first available site when current siteId is null`() {
    assertEquals(
      "site-1",
      resolvedSiteId(currentSiteId = null, availableSiteIds = listOf("site-1", "site-2")),
    )
  }

  @Test
  fun `returns null when available sites is empty`() {
    assertNull(resolvedSiteId(currentSiteId = "site-1", availableSiteIds = emptyList()))
  }

  @Test
  fun `returns null when current is null and available sites is empty`() {
    assertNull(resolvedSiteId(currentSiteId = null, availableSiteIds = emptyList()))
  }
}
