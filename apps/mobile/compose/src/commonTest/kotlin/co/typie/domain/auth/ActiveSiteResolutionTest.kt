package co.typie.domain.auth

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ActiveSiteResolutionTest {
  @Test
  fun keepsStoredSiteIdWhenAvailable() {
    assertEquals("S2", resolveActiveSiteId("S2", listOf("S1", "S2")))
  }

  @Test
  fun fallsBackToFirstSiteWhenStoredIsInvalid() {
    assertEquals("S1", resolveActiveSiteId("GONE", listOf("S1", "S2")))
  }

  @Test
  fun fallsBackToFirstSiteWhenStoredIsNull() {
    assertEquals("S1", resolveActiveSiteId(null, listOf("S1", "S2")))
  }

  @Test
  fun returnsNullWhenNoSitesAvailable() {
    assertNull(resolveActiveSiteId("S1", emptyList()))
    assertNull(resolveActiveSiteId(null, emptyList()))
  }
}
