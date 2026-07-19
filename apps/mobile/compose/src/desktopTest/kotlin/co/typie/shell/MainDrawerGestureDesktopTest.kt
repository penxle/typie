package co.typie.shell

import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.performTrackpadInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ui.component.drawer.Drawer
import co.typie.ui.component.drawer.DrawerAnchor
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class MainDrawerGestureDesktopTest {
  @Test
  fun `slow full area drag below halfway closes`() = runComposeUiTest {
    lateinit var drawer: Drawer

    setContent {
      drawer = rememberTestDrawer()
      DrawerSwipeHost(drawer)
    }
    waitForIdle()

    onNodeWithTag(SwipeHostTag).performTouchInput {
      down(Offset(x = 20f, y = center.y))
      repeat(10) { moveBy(Offset(x = 12f, y = 0f), delayMillis = 50L) }
      up()
    }

    waitUntil { abs(drawer.state.requireOffset() + 300f) < 0.1f }
    assertEquals(DrawerAnchor.Closed, drawer.state.currentValue)
  }

  @Test
  fun `slow full area drag past halfway opens`() = runComposeUiTest {
    lateinit var drawer: Drawer

    setContent {
      drawer = rememberTestDrawer()
      DrawerSwipeHost(drawer)
    }
    waitForIdle()

    onNodeWithTag(SwipeHostTag).performTouchInput {
      down(Offset(x = 20f, y = center.y))
      repeat(15) { moveBy(Offset(x = 12f, y = 0f), delayMillis = 50L) }
      up()
    }

    waitUntil { abs(drawer.state.requireOffset()) < 0.1f }
    assertEquals(DrawerAnchor.Open, drawer.state.currentValue)
  }

  @Test
  fun `fast full area release opens below halfway`() = runComposeUiTest {
    lateinit var drawer: Drawer
    var touchSlop = 0f

    setContent {
      drawer = rememberTestDrawer()
      touchSlop = LocalViewConfiguration.current.touchSlop
      DrawerSwipeHost(drawer)
    }
    waitForIdle()

    onNodeWithTag(SwipeHostTag).performTouchInput {
      down(Offset(x = 20f, y = center.y))
      moveBy(Offset(x = touchSlop + 1f, y = 0f), delayMillis = 100L)
      repeat(6) { moveBy(Offset(x = 10f, y = 0f), delayMillis = 1L) }
      up()
    }

    waitUntil { abs(drawer.state.requireOffset()) < 0.1f }
    assertEquals(DrawerAnchor.Open, drawer.state.currentValue)
  }

  @Test
  fun `vertical dominant full area gesture stays rejected`() = runComposeUiTest {
    lateinit var drawer: Drawer
    var touchSlop = 0f

    setContent {
      drawer = rememberTestDrawer()
      touchSlop = LocalViewConfiguration.current.touchSlop
      DrawerSwipeHost(drawer)
    }
    waitForIdle()

    onNodeWithTag(SwipeHostTag).performTouchInput {
      down(center)
      moveBy(Offset(x = 0f, y = touchSlop + 20f))
      moveBy(Offset(x = 200f, y = 0f))
      up()
    }
    waitForIdle()

    assertEquals(-300f, drawer.state.requireOffset(), absoluteTolerance = 0.1f)
  }

  @Test
  fun `horizontal pager touch drag owns the gesture`() = runComposeUiTest {
    lateinit var drawer: Drawer
    lateinit var pagerState: PagerState

    setContent {
      drawer = rememberTestDrawer()
      pagerState = rememberPagerState(initialPage = 1, pageCount = { 2 })
      DrawerSwipeHost(drawer) { TestPager(pagerState) }
    }
    waitForIdle()

    onNodeWithTag(PagerTag).performTouchInput {
      down(center)
      repeat(8) { moveBy(Offset(x = 20f, y = 0f), delayMillis = 16L) }
      up()
    }
    waitUntil { !pagerState.isScrollInProgress }

    assertEquals(-300f, drawer.state.requireOffset(), absoluteTolerance = 0.1f)
    assertEquals(0, pagerState.currentPage)
  }

  @Test
  fun `mouse drag over horizontal pager never reveals the full area drawer`() = runComposeUiTest {
    lateinit var drawer: Drawer
    lateinit var pagerState: PagerState
    var touchSlop = 0f

    setContent {
      drawer = rememberTestDrawer()
      pagerState = rememberPagerState(initialPage = 1, pageCount = { 2 })
      touchSlop = LocalViewConfiguration.current.touchSlop
      DrawerSwipeHost(drawer) { TestPager(pagerState) }
    }
    waitForIdle()
    val pager = onNodeWithTag(PagerTag)

    pager.performTrackpadInput {
      moveTo(center)
      press()
      moveBy(Offset(x = touchSlop + 5f, y = 0f), delayMillis = 16L)
      moveBy(Offset(x = 180f, y = 0f), delayMillis = 16L)
    }
    waitForIdle()
    assertEquals(-300f, drawer.state.requireOffset(), absoluteTolerance = 0.1f)

    pager.performTrackpadInput {
      moveBy(Offset(x = -220f, y = 0f), delayMillis = 16L)
      release()
    }
    waitForIdle()

    assertEquals(-300f, drawer.state.requireOffset(), absoluteTolerance = 0.1f)
    assertEquals(1, pagerState.currentPage)
  }

  @Composable
  private fun rememberTestDrawer(): Drawer = remember { Drawer().also { it.updateTestAnchors() } }

  @Composable
  private fun DrawerSwipeHost(drawer: Drawer, content: @Composable () -> Unit = {}) {
    Box(
      Modifier.size(width = 320.dp, height = 640.dp)
        .testTag(SwipeHostTag)
        .then(mainDrawerSwipeToOpenModifier(drawer, enabled = true))
    ) {
      content()
    }
  }

  @Composable
  private fun TestPager(state: PagerState) {
    HorizontalPager(state = state, modifier = Modifier.fillMaxSize().testTag(PagerTag)) {
      Box(Modifier.fillMaxSize())
    }
  }

  private fun Drawer.updateTestAnchors() {
    state.updateAnchors(
      DraggableAnchors {
        DrawerAnchor.Closed at -300f
        DrawerAnchor.Open at 0f
      },
      DrawerAnchor.Closed,
    )
  }

  private companion object {
    const val PagerTag = "main-drawer-horizontal-pager"
    const val SwipeHostTag = "main-drawer-swipe-host"
  }
}
