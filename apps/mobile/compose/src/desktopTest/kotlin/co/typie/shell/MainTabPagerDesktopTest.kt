package co.typie.shell

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

@OptIn(ExperimentalTestApi::class)
class MainTabPagerDesktopTest {
  @Test
  fun `idle pager composes only the visible body`() = runComposeUiTest {
    val composedTabs = mutableSetOf<Tab>()

    setContent {
      val state = rememberMainTabState()
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = true,
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
  fun `partial drag moves adjacent bodies while direct motion keeps chrome on its origin`() =
    runComposeUiTest {
      lateinit var state: MainTabState
      val pageLefts = mutableMapOf<Tab, Float>()

      setContent {
        state = rememberMainTabState()
        Box(Modifier.size(width = 320.dp, height = 640.dp)) {
          MainTabPager(
            state = state,
            gestureAdmissionAllowed = true,
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

      waitUntil {
        (pageLefts[Tab.Home] ?: 0f) < -1f && state.motion?.source == MainTabMotionSource.DirectDrag
      }
      val homeLeft = pageLefts.getValue(Tab.Home)
      val spaceLeft = pageLefts.getValue(Tab.Space)
      assertTrue(homeLeft < -1f)
      assertTrue(spaceLeft in 1f..319f)
      assertEquals(320f, spaceLeft - homeLeft, absoluteTolerance = 1f)
      assertEquals(Tab.Home, mainTabChromeTab(state.settledTab, state.motion))
      assertEquals(
        chromeLeft,
        onNodeWithTag(ChromeTag).fetchSemanticsNode().boundsInRoot.left,
        absoluteTolerance = 0.1f,
      )

      onNodeWithTag(PagerTag).performTouchInput { up() }
      waitUntil { state.motion == null }
    }

  @Test
  fun `disabled pager is transparent to descendant side effects`() = runComposeUiTest {
    lateinit var state: MainTabState
    lateinit var nestedScrollDispatcher: NestedScrollDispatcher

    setContent {
      state = rememberMainTabState()
      nestedScrollDispatcher = remember { NestedScrollDispatcher() }
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = false,
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
    assertEquals(0f, state.pagerState.currentPageOffsetFraction)
  }

  @Test
  fun `raw same page work never creates main tab motion`() = runComposeUiTest {
    lateinit var state: MainTabState
    lateinit var releaseScroll: CompletableDeferred<Unit>
    lateinit var scrollJob: Job
    lateinit var startScroll: () -> Unit

    setContent {
      state = rememberMainTabState()
      val scope = rememberCoroutineScope()
      startScroll = {
        releaseScroll = CompletableDeferred()
        scrollJob = scope.launch { state.pagerState.scroll { releaseScroll.await() } }
      }
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = false,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    runOnIdle { startScroll() }
    waitUntil { state.pagerState.isScrollInProgress }
    runOnIdle {
      assertNull(state.motion)
      assertEquals(Tab.Home, state.settledTab)
      assertEquals(Tab.Home.ordinal.toFloat(), state.bodyPosition)
      releaseScroll.complete(Unit)
    }
    waitUntil { scrollJob.isCompleted && !state.pagerState.isScrollInProgress }
    assertNull(state.motion)
  }

  @Test
  fun `late raw page motion is rejected after nested navigation locks`() = runComposeUiTest {
    lateinit var state: MainTabState
    lateinit var scrollJob: Job
    lateinit var startScroll: () -> Unit

    setContent {
      state = rememberMainTabState()
      val scope = rememberCoroutineScope()
      startScroll = {
        scrollJob = scope.launch { state.pagerState.animateScrollToPage(Tab.Space.ordinal) }
      }
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = false,
        navigationLocked = true,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    runOnIdle { startScroll() }
    waitUntil { scrollJob.isCompleted && !state.pagerState.isScrollInProgress }

    assertNull(state.motion)
    assertEquals(Tab.Home, state.settledTab)
    assertEquals(Tab.Home.ordinal, state.pagerState.settledPage)
    assertEquals(0f, state.pagerState.currentPageOffsetFraction)
  }

  @Test
  fun `navigation lock cancels a held direct drag to its origin`() = runComposeUiTest {
    lateinit var state: MainTabState
    var navigationLocked by mutableStateOf(false)

    setContent {
      state = rememberMainTabState()
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = !navigationLocked,
        navigationLocked = navigationLocked,
        modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(PagerTag),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    onNodeWithTag(PagerTag).performTouchInput {
      down(center)
      moveBy(Offset(x = -80f, y = 0f), delayMillis = 100L)
    }
    waitUntil { state.motion?.source == MainTabMotionSource.DirectDrag }

    runOnIdle { navigationLocked = true }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null }

    assertEquals(Tab.Home, state.settledTab)
    assertEquals(Tab.Home.ordinal, state.pagerState.settledPage)
    assertEquals(0f, state.pagerState.currentPageOffsetFraction)
    onNodeWithTag(PagerTag).performTouchInput { up() }
  }

  @Test
  fun `rapid programmatic selection settles on the latest request`() = runComposeUiTest {
    lateinit var state: MainTabState

    setContent {
      state = rememberMainTabState()
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = true,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    runOnIdle { state.selectTab(Tab.Notes) }
    waitUntil { state.motion?.target == Tab.Notes }
    runOnIdle { state.selectTab(Tab.Space) }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null && state.settledTab == Tab.Space }

    assertEquals(Tab.Space.ordinal, state.pagerState.settledPage)
  }

  @Test
  fun `cancel during owned motion returns to its origin`() = runComposeUiTest {
    lateinit var state: MainTabState

    setContent {
      state = rememberMainTabState()
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = true,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    mainClock.autoAdvance = false
    runOnIdle { state.selectTab(Tab.Notes) }
    mainClock.advanceTimeByFrame()
    runOnIdle { assertEquals(Tab.Notes, state.motion?.target) }
    runOnIdle { state.cancelToOrigin() }
    mainClock.autoAdvance = true
    waitUntil(timeoutMillis = 5_000L) { state.motion == null }

    assertEquals(Tab.Home, state.settledTab)
    assertEquals(Tab.Home.ordinal, state.pagerState.settledPage)
  }

  @Test
  fun `pager accessibility scroll creates committed motion and settles`() = runComposeUiTest {
    lateinit var state: MainTabState

    setContent {
      state = rememberMainTabState()
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = true,
        modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(PagerTag),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    onNodeWithTag(PagerTag).performSemanticsAction(SemanticsActions.PageRight) { action ->
      assertTrue(action())
    }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null && state.settledTab != Tab.Home }

    assertEquals(Tab.Space, state.settledTab)
  }

  @Test
  fun `late pager accessibility scroll is rejected after admission closes`() = runComposeUiTest {
    lateinit var state: MainTabState
    lateinit var lateAccessibilityPageRight: () -> Boolean
    var gestureAdmissionAllowed by mutableStateOf(true)

    setContent {
      state = rememberMainTabState()
      MainTabPager(
        state = state,
        gestureAdmissionAllowed = gestureAdmissionAllowed,
        modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(PagerTag),
      ) {
        Box(Modifier.fillMaxSize())
      }
    }
    waitForIdle()

    onNodeWithTag(PagerTag).performSemanticsAction(SemanticsActions.PageRight) { action ->
      lateAccessibilityPageRight = action
    }
    runOnIdle { gestureAdmissionAllowed = false }
    waitForIdle()
    runOnIdle { assertTrue(lateAccessibilityPageRight()) }
    waitUntil { !state.pagerState.isScrollInProgress }

    assertNull(state.motion)
    assertEquals(Tab.Home, state.settledTab)
    assertEquals(Tab.Home.ordinal, state.pagerState.settledPage)
  }

  @Test
  fun `child pager keeps an outward edge drag from the main pager`() = runComposeUiTest {
    lateinit var mainTabState: MainTabState
    lateinit var childPagerState: PagerState

    setContent {
      mainTabState = rememberMainTabState()
      childPagerState = rememberPagerState(initialPage = 1, pageCount = { 2 })
      MainTabPager(
        state = mainTabState,
        gestureAdmissionAllowed = true,
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
    waitUntil { !childPagerState.isScrollInProgress && !mainTabState.pagerState.isScrollInProgress }

    assertEquals(1, childPagerState.settledPage)
    assertEquals(Tab.Home, mainTabState.settledTab)
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
