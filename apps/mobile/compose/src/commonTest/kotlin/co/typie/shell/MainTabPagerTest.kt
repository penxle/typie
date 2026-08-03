package co.typie.shell

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class MainTabPagerTest {
  @Test
  fun `tab activation weights follow fractional pager position`() {
    assertEquals(
      mapOf(Tab.Home to 1f, Tab.Space to 0f, Tab.Notes to 0f),
      mainTabActivationWeights(0f),
    )
    assertEquals(
      mapOf(Tab.Home to 0.75f, Tab.Space to 0.25f, Tab.Notes to 0f),
      mainTabActivationWeights(0.25f),
    )
    assertEquals(
      mapOf(Tab.Home to 0f, Tab.Space to 1f, Tab.Notes to 0f),
      mainTabActivationWeights(1f),
    )
    assertEquals(
      mapOf(Tab.Home to 0f, Tab.Space to 0.5f, Tab.Notes to 0.5f),
      mainTabActivationWeights(1.5f),
    )
    assertEquals(
      mapOf(Tab.Home to 0f, Tab.Space to 0f, Tab.Notes to 1f),
      mainTabActivationWeights(2f),
    )
  }

  @Test
  fun `chrome stays settled during direct drag and follows committed target while settling`() {
    assertEquals(
      Tab.Home,
      resolveMainTabChrome(settledTab = Tab.Home, targetTab = Tab.Space, isDirectDrag = true),
    )
    assertEquals(
      Tab.Space,
      resolveMainTabChrome(settledTab = Tab.Home, targetTab = Tab.Space, isDirectDrag = false),
    )
    assertEquals(
      Tab.Home,
      resolveMainTabChrome(settledTab = Tab.Home, targetTab = Tab.Home, isDirectDrag = false),
    )
  }

  @Test
  fun `new pager gesture admission requires root idle pager and drawer`() {
    assertTrue(
      canAdmitMainTabPagerGesture(
        navigatorCanPop = false,
        navigatorIsTransitioning = false,
        pagerAtRest = true,
        drawerAtRest = true,
      )
    )
    assertFalse(
      canAdmitMainTabPagerGesture(
        navigatorCanPop = true,
        navigatorIsTransitioning = false,
        pagerAtRest = true,
        drawerAtRest = true,
      )
    )
    assertFalse(
      canAdmitMainTabPagerGesture(
        navigatorCanPop = false,
        navigatorIsTransitioning = true,
        pagerAtRest = true,
        drawerAtRest = true,
      )
    )
    assertFalse(
      canAdmitMainTabPagerGesture(
        navigatorCanPop = false,
        navigatorIsTransitioning = false,
        pagerAtRest = false,
        drawerAtRest = true,
      )
    )
    assertFalse(
      canAdmitMainTabPagerGesture(
        navigatorCanPop = false,
        navigatorIsTransitioning = false,
        pagerAtRest = true,
        drawerAtRest = false,
      )
    )
  }

  @Test
  fun `bottom pill handoff follows pager travel including non adjacent selection`() {
    assertEquals(
      0.25f,
      mainTabPillHandoffProgress(position = 0.5f, originPosition = 0f, targetPosition = 2f),
    )
    assertEquals(
      0.5f,
      mainTabPillHandoffProgress(position = 1.5f, originPosition = 1f, targetPosition = 2f),
    )
    assertEquals(
      0.5f,
      mainTabPillHandoffProgress(position = 0.5f, originPosition = 1f, targetPosition = 0f),
    )
    assertEquals(
      1f,
      mainTabPillHandoffProgress(position = 1f, originPosition = 1f, targetPosition = 1f),
    )
  }
}
