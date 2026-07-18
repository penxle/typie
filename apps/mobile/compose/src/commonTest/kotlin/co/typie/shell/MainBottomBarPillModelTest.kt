package co.typie.shell

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest

class MainBottomBarPillModelTest {

  @Test
  fun `main tabs expose home space and notes`() {
    assertEquals(listOf(Tab.Home, Tab.Space, Tab.Notes), Tab.entries)
  }

  @Test
  fun cumulativeCenters_returnsHalfWidthForFirstAndOffsetForRest() {
    val widths = mapOf(Tab.Home to 120f, Tab.Space to 64f, Tab.Notes to 64f)
    val centers = cumulativeCenters(widths)

    assertEquals(60f, centers.getValue(Tab.Home))
    assertEquals(152f, centers.getValue(Tab.Space))
    assertEquals(216f, centers.getValue(Tab.Notes))
  }

  @Test
  fun cumulativeCenters_handlesSwappedActiveWidths() {
    val widths = mapOf(Tab.Home to 64f, Tab.Space to 120f, Tab.Notes to 64f)
    val centers = cumulativeCenters(widths)

    assertEquals(32f, centers.getValue(Tab.Home))
    assertEquals(124f, centers.getValue(Tab.Space))
    assertEquals(216f, centers.getValue(Tab.Notes))
  }

  @Test
  fun nearestTab_picksClosestCenterToPointer() {
    val centers = mapOf(Tab.Home to 60f, Tab.Space to 152f, Tab.Notes to 216f)

    assertEquals(Tab.Home, nearestTab(centers, totalWidth = 248f, pointerX = 0f))
    assertEquals(Tab.Home, nearestTab(centers, totalWidth = 248f, pointerX = 105f))
    assertEquals(Tab.Space, nearestTab(centers, totalWidth = 248f, pointerX = 107f))
    assertEquals(Tab.Space, nearestTab(centers, totalWidth = 248f, pointerX = 184f))
    assertEquals(Tab.Notes, nearestTab(centers, totalWidth = 248f, pointerX = 186f))
    assertEquals(Tab.Notes, nearestTab(centers, totalWidth = 248f, pointerX = 248f))
  }

  @Test
  fun nearestTab_clampsPointerOutsideTrack() {
    val centers = mapOf(Tab.Home to 60f, Tab.Space to 152f, Tab.Notes to 216f)

    assertEquals(Tab.Home, nearestTab(centers, totalWidth = 248f, pointerX = -50f))
    assertEquals(Tab.Notes, nearestTab(centers, totalWidth = 248f, pointerX = 500f))
  }

  @Test
  fun followDrag_usesResolvedFullStretchDistance() = runTest {
    val state = MainBottomBarPillState(scope = this, initialActiveTab = Tab.Home)

    state.followDrag(previousX = 0f, targetX = 18f, totalWidth = 100f, fullStretchDelta = 36f)

    assertEquals(0.5f, state.deformationTarget)
  }
}
