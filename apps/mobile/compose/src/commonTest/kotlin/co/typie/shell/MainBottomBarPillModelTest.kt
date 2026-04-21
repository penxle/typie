package co.typie.shell

import kotlin.test.Test
import kotlin.test.assertEquals

class MainBottomBarPillModelTest {

  @Test
  fun cumulativeCenters_returnsHalfWidthForFirstAndOffsetForRest() {
    val widths = mapOf(Tab.Home to 120f, Tab.Space to 64f)
    val centers = cumulativeCenters(widths)

    assertEquals(60f, centers.getValue(Tab.Home))
    assertEquals(152f, centers.getValue(Tab.Space))
  }

  @Test
  fun cumulativeCenters_handlesSwappedActiveWidths() {
    val widths = mapOf(Tab.Home to 64f, Tab.Space to 120f)
    val centers = cumulativeCenters(widths)

    assertEquals(32f, centers.getValue(Tab.Home))
    assertEquals(124f, centers.getValue(Tab.Space))
  }

  @Test
  fun nearestTab_picksClosestCenterToPointer() {
    val centers = mapOf(Tab.Home to 60f, Tab.Space to 152f)

    assertEquals(Tab.Home, nearestTab(centers, totalWidth = 184f, pointerX = 0f))
    assertEquals(Tab.Home, nearestTab(centers, totalWidth = 184f, pointerX = 105f))
    assertEquals(Tab.Space, nearestTab(centers, totalWidth = 184f, pointerX = 107f))
    assertEquals(Tab.Space, nearestTab(centers, totalWidth = 184f, pointerX = 184f))
  }

  @Test
  fun nearestTab_clampsPointerOutsideTrack() {
    val centers = mapOf(Tab.Home to 60f, Tab.Space to 152f)

    assertEquals(Tab.Home, nearestTab(centers, totalWidth = 184f, pointerX = -50f))
    assertEquals(Tab.Space, nearestTab(centers, totalWidth = 184f, pointerX = 500f))
  }
}
