package co.typie.shell

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

@OptIn(ExperimentalTestApi::class)
class MainTabPagerDesktopTest {
  @Test
  fun `idle pager composes only the visible body`() = runComposeUiTest {
    val composedTabs = mutableSetOf<Tab>()

    setContent {
      val pagerState = rememberPagerState(pageCount = { Tab.entries.size })
      MainTabPager(
        state = pagerState,
        userScrollEnabled = true,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { tab ->
        DisposableEffect(tab) {
          composedTabs += tab
          onDispose { composedTabs -= tab }
        }
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    runOnIdle { assertEquals(setOf(Tab.Home), composedTabs) }
  }

  @Test
  fun `partial drag moves adjacent bodies while fixed chrome stays in place`() = runComposeUiTest {
    lateinit var pagerState: PagerState
    val pageLefts = mutableMapOf<Tab, Float>()

    setContent {
      pagerState = rememberPagerState(pageCount = { Tab.entries.size })
      Box(Modifier.size(width = 320.dp, height = 640.dp)) {
        MainTabPager(
          state = pagerState,
          userScrollEnabled = true,
          modifier = Modifier.fillMaxSize().testTag(PagerTag),
        ) { tab ->
          Box(
            Modifier.fillMaxSize()
              .onGloballyPositioned { pageLefts[tab] = it.positionInRoot().x }
              .testTag("page-${tab.name}")
          )
        }
        Box(Modifier.size(width = 320.dp, height = 56.dp).testTag(ChromeTag))
      }
    }
    waitForIdle()

    val chromeLeft = onNodeWithTag(ChromeTag).fetchSemanticsNode().boundsInRoot.left
    onNodeWithTag(PagerTag).performTouchInput {
      down(center)
      moveBy(Offset(x = -80f, y = 0f), delayMillis = 100L)
    }

    waitUntil { (pageLefts[Tab.Home] ?: 0f) < -1f }
    val homeLeft = pageLefts.getValue(Tab.Home)
    val spaceLeft = pageLefts.getValue(Tab.Space)
    assertTrue(homeLeft < -1f)
    assertTrue(spaceLeft in 1f..319f)
    assertEquals(320f, spaceLeft - homeLeft, absoluteTolerance = 1f)
    assertEquals(
      chromeLeft,
      onNodeWithTag(ChromeTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.1f,
    )

    onNodeWithTag(PagerTag).performTouchInput { up() }
    waitUntil { !pagerState.isScrollInProgress }
  }

  @Test
  fun `disabled pager is transparent to descendant side effects`() = runComposeUiTest {
    lateinit var pagerState: PagerState
    lateinit var nestedScrollDispatcher: NestedScrollDispatcher

    setContent {
      pagerState = rememberPagerState(pageCount = { Tab.entries.size })
      nestedScrollDispatcher = remember { NestedScrollDispatcher() }
      MainTabPager(
        state = pagerState,
        userScrollEnabled = false,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { tab ->
        Box(
          Modifier.fillMaxSize()
            .then(
              if (tab == Tab.Home) {
                Modifier.nestedScroll(NoOpNestedScrollConnection, nestedScrollDispatcher)
              } else {
                Modifier
              }
            )
        )
      }
    }
    waitForIdle()

    var postScrollConsumed = Offset.Unspecified
    var postFlingConsumed = Velocity(Float.NaN, Float.NaN)
    runOnIdle {
      postScrollConsumed =
        nestedScrollDispatcher.dispatchPostScroll(
          consumed = Offset(x = 0f, y = -12f),
          available = Offset(x = -8f, y = 0f),
          source = NestedScrollSource.SideEffect,
        )
      postFlingConsumed = runBlocking {
        nestedScrollDispatcher.dispatchPostFling(
          consumed = Velocity(x = 0f, y = -200f),
          available = Velocity(x = -100f, y = 0f),
        )
      }
    }

    assertEquals(Offset.Zero, postScrollConsumed)
    assertEquals(Velocity.Zero, postFlingConsumed)
    assertEquals(0f, pagerState.currentPageOffsetFraction)
  }

  @Test
  fun `pager motion that starts after nested navigation is cancelled before another tab settles`() =
    runComposeUiTest {
      lateinit var pagerState: PagerState
      lateinit var scrollJob: Job
      lateinit var startScroll: () -> Unit

      setContent {
        pagerState = rememberPagerState(pageCount = { Tab.entries.size })
        val scope = rememberCoroutineScope()
        startScroll = {
          scrollJob = scope.launch { pagerState.animateScrollToPage(Tab.Space.ordinal) }
        }
        MainTabPagerNavigationGuard(state = pagerState, navigationLocked = true, onInterrupt = {})
        MainTabPager(
          state = pagerState,
          userScrollEnabled = true,
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) {
          Box(Modifier.fillMaxSize())
        }
      }
      waitForIdle()

      runOnIdle { startScroll() }
      waitUntil { scrollJob.isCompleted }

      assertEquals(Tab.Home.ordinal, pagerState.settledPage)
      assertEquals(0f, pagerState.currentPageOffsetFraction)
    }

  @Test
  fun `child pager keeps an outward edge drag from the main pager`() = runComposeUiTest {
    lateinit var mainPagerState: PagerState
    lateinit var childPagerState: PagerState

    setContent {
      mainPagerState = rememberPagerState(pageCount = { Tab.entries.size })
      childPagerState = rememberPagerState(initialPage = 1, pageCount = { 2 })
      MainTabPager(
        state = mainPagerState,
        userScrollEnabled = true,
        modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(PagerTag),
      ) { tab ->
        if (tab == Tab.Home) ChildPager(childPagerState) else Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    onNodeWithTag(ChildPagerTag).performTouchInput {
      down(center)
      repeat(8) { moveBy(Offset(x = -20f, y = 0f), delayMillis = 16L) }
      up()
    }
    waitUntil { !childPagerState.isScrollInProgress && !mainPagerState.isScrollInProgress }

    assertEquals(1, childPagerState.settledPage)
    assertEquals(0, mainPagerState.settledPage)
  }

  @Composable
  private fun ChildPager(state: PagerState) {
    HorizontalPager(
      state = state,
      modifier = Modifier.fillMaxSize().mainTabPagerChildHorizontalGesture().testTag(ChildPagerTag),
    ) {
      Box(Modifier.fillMaxSize())
    }
  }

  private companion object {
    const val ChromeTag = "main-tab-fixed-chrome"
    const val ChildPagerTag = "main-tab-child-pager"
    const val PagerTag = "main-tab-pager"
    val NoOpNestedScrollConnection = object : NestedScrollConnection {}
  }
}
