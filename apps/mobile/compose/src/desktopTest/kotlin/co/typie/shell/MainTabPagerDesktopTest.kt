package co.typie.shell

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
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
import co.typie.navigation.NavigationResult
import co.typie.navigation.NavigationStackTestHost
import co.typie.navigation.Navigator
import co.typie.navigation.RouteRemovalDecision
import co.typie.navigation.RouteRemovalInterceptor
import co.typie.navigation.RouteRemovalPreparation
import co.typie.platform.LocalSoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationDriver
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import co.typie.route.Route
import co.typie.ui.component.topbar.TopBarState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotSame
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking

@OptIn(ExperimentalTestApi::class)
class MainTabPagerDesktopTest {
  @Test
  fun `focused nested route pop restores pager gesture admission`() = runComposeUiTest {
    val navigator = Navigator(listOf(Route.Home, Route.SpaceSettings))
    val focusRequester = FocusRequester()
    lateinit var mainTabState: MainTabState
    lateinit var pop: () -> Unit
    var inputFocused = false
    var outgoingAttached = false
    var destinationAppliedWhileOutgoingAttached = false

    setContent {
      val scope = rememberCoroutineScope()
      pop = { scope.launch { navigator.pop() } }
      MainTabPager(
        state = rememberMainTabState().also { mainTabState = it },
        gestureAdmissionAllowed = !navigator.canPop && !navigator.isTransitioning,
        modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(PagerTag),
      ) { tab ->
        if (tab == Tab.Home) {
          NavigationStackTestHost(
            navigator = navigator,
            topBarState = remember { TopBarState() },
            modifier = Modifier.fillMaxSize(),
          ) { route ->
            if (route == Route.SpaceSettings) {
              DisposableEffect(route) {
                outgoingAttached = true
                onDispose { outgoingAttached = false }
              }
              BasicTextField(
                value = "",
                onValueChange = {},
                modifier =
                  Modifier.fillMaxSize().focusRequester(focusRequester).onFocusChanged {
                    inputFocused = it.isFocused
                  },
              )
              LaunchedEffect(route) { focusRequester.requestFocus() }
            } else {
              SideEffect {
                if (
                  navigator.current == Route.Home && navigator.isTransitioning && outgoingAttached
                ) {
                  destinationAppliedWhileOutgoingAttached = true
                }
              }
              Box(Modifier.fillMaxSize())
            }
          }
        } else {
          Box(Modifier.fillMaxSize())
        }
      }
    }
    waitUntil(timeoutMillis = 5_000L) {
      navigator.current == Route.SpaceSettings && !navigator.isTransitioning
    }
    waitUntil { inputFocused }

    runOnIdle { pop() }
    waitUntil(timeoutMillis = 5_000L) {
      navigator.current == Route.Home && !navigator.isTransitioning
    }
    assertTrue(destinationAppliedWhileOutgoingAttached)
    assertFalse(outgoingAttached)

    onNodeWithTag(PagerTag).performTouchInput {
      down(center)
      repeat(10) { moveBy(Offset(x = -20f, y = 0f), delayMillis = 16L) }
      up()
    }
    waitUntil(timeoutMillis = 5_000L) { mainTabState.motion == null }
    assertEquals(Tab.Space, mainTabState.settledTab)
  }

  @Test
  fun `disposing navigation stack during scene handoff clears removed route store`() =
    runComposeUiTest {
      val navigator = Navigator(listOf(Route.Home, Route.SpaceSettings))
      val removedRouteStore = navigator.viewModelStoreFor(Route.SpaceSettings)
      lateinit var pop: () -> Unit
      var showNavigationStack by mutableStateOf(true)
      var outgoingAttached = false

      setContent {
        val scope = rememberCoroutineScope()
        pop = { scope.launch { runCatching { navigator.pop() } } }
        MainTabPager(
          state = rememberMainTabState(),
          gestureAdmissionAllowed = !navigator.canPop && !navigator.isTransitioning,
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) { tab ->
          if (tab == Tab.Home && showNavigationStack) {
            NavigationStackTestHost(
              navigator = navigator,
              topBarState = remember { TopBarState() },
              modifier = Modifier.fillMaxSize(),
            ) { route ->
              if (route == Route.SpaceSettings) {
                DisposableEffect(route) {
                  outgoingAttached = true
                  onDispose { outgoingAttached = false }
                }
              } else {
                SideEffect {
                  if (navigator.current == Route.Home && outgoingAttached) {
                    showNavigationStack = false
                  }
                }
              }
              Box(Modifier.fillMaxSize())
            }
          } else {
            Box(Modifier.fillMaxSize())
          }
        }
      }
      waitUntil { outgoingAttached }

      runOnIdle { pop() }
      waitUntil(timeoutMillis = 5_000L) { !showNavigationStack }
      waitUntil(timeoutMillis = 5_000L) { !navigator.isTransitioning }

      assertEquals(Route.Home, navigator.current)
      assertNotSame(removedRouteStore, navigator.viewModelStoreFor(Route.SpaceSettings))
    }

  @Test
  fun `disposing navigation stack during bypass removal completes transition`() = runComposeUiTest {
    val editorRoute = Route.Editor("editor")
    val documentRoute = Route.Document("document")
    val navigator = Navigator(listOf(Route.Home, editorRoute, documentRoute))
    lateinit var removeDocument: () -> Unit
    var showNavigationStack by mutableStateOf(true)
    var removalResult: Result<NavigationResult>? = null

    navigator.routeRemovals.register(
      editorRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation =
          RouteRemovalPreparation.Ready

        override suspend fun resolveDecision(): RouteRemovalDecision =
          error("Ready removal must not require a decision")

        override suspend fun rollback() = Unit
      },
    )

    setContent {
      val scope = rememberCoroutineScope()
      removeDocument = {
        scope.launch {
          val prepared = checkNotNull(navigator.prepareAdjacentRemoval(documentRoute, editorRoute))
          removalResult = runCatching { navigator.commitAdjacentRemoval(prepared) }
        }
      }
      MainTabPager(
        state = rememberMainTabState(),
        gestureAdmissionAllowed = !navigator.canPop && !navigator.isTransitioning,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { tab ->
        if (tab == Tab.Home && showNavigationStack) {
          NavigationStackTestHost(
            navigator = navigator,
            topBarState = remember { TopBarState() },
            modifier = Modifier.fillMaxSize(),
          ) { route ->
            Box(
              Modifier.fillMaxSize()
                .testTag(
                  if (route == documentRoute) BypassOutgoingRouteTag else "bypass-background"
                )
            )
          }
        } else {
          Box(Modifier.fillMaxSize())
        }
      }
    }
    val outgoingRoute = onNodeWithTag(BypassOutgoingRouteTag)
    outgoingRoute.fetchSemanticsNode()

    try {
      mainClock.autoAdvance = false
      runOnIdle { removeDocument() }
      waitUntil(timeoutMillis = 5_000L) { navigator.popRequested }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeBy(200L)
      assertTrue(outgoingRoute.fetchSemanticsNode().boundsInRoot.top > 0f)
      runOnIdle { showNavigationStack = false }
      mainClock.advanceTimeByFrame()
    } finally {
      mainClock.autoAdvance = true
    }
    waitUntil(timeoutMillis = 5_000L) { !navigator.isTransitioning }
    waitUntil(timeoutMillis = 5_000L) { removalResult != null }

    assertEquals(Route.Home, navigator.current)
    assertEquals(NavigationResult.ReachedTarget, removalResult?.getOrThrow())
  }

  @Test
  fun `committed tab swipe clears focus only after keyboard hide is accepted`() = runComposeUiTest {
    lateinit var state: MainTabState
    val focusRequester = FocusRequester()
    val keyboardDriver = DeferredMainTabKeyboardDriver()
    val controller = SoftwareKeyboardPresentationController { keyboardDriver }
    var inputFocused = false

    setContent {
      CompositionLocalProvider(LocalSoftwareKeyboardPresentationController provides controller) {
        Box(Modifier.size(width = 320.dp, height = 640.dp)) {
          MainTabPager(
            state = rememberMainTabState().also { state = it },
            gestureAdmissionAllowed = true,
            modifier = Modifier.fillMaxSize().testTag(PagerTag),
          ) {
            Box(Modifier.fillMaxSize())
          }
          BasicTextField(
            value = "",
            onValueChange = {},
            modifier =
              Modifier.size(1.dp).focusRequester(focusRequester).onFocusChanged {
                inputFocused = it.isFocused
              },
          )
          LaunchedEffect(Unit) { focusRequester.requestFocus() }
        }
      }
    }
    waitUntil { inputFocused }

    onNodeWithTag(PagerTag).performTouchInput {
      down(center)
      repeat(10) { moveBy(Offset(x = -20f, y = 0f), delayMillis = 16L) }
    }
    waitUntil { state.motion?.source == MainTabMotionSource.DirectDrag }
    runOnIdle {
      assertTrue(inputFocused)
      assertTrue(keyboardDriver.endpoints.isEmpty())
      assertTrue(keyboardDriver.progress.any { it > 0f })
      assertTrue(keyboardDriver.progress.all { it in 0f..1f })
    }

    onNodeWithTag(PagerTag).performTouchInput { up() }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null && state.settledTab == Tab.Space }
    waitUntil { keyboardDriver.endpoints == listOf(SoftwareKeyboardPresentationEndpoint.Hidden) }

    runOnIdle {
      assertTrue(inputFocused)
      assertEquals(1f, keyboardDriver.progress.last())
    }
    runOnIdle { keyboardDriver.acceptEndpoint() }
    waitUntil { !inputFocused }
  }

  @Test
  fun `delayed keyboard hide enables focus for the latest settled tab`() = runComposeUiTest {
    lateinit var state: MainTabState
    val keyboardDriver = DeferredMainTabKeyboardDriver()
    val controller = SoftwareKeyboardPresentationController { keyboardDriver }

    setContent {
      CompositionLocalProvider(LocalSoftwareKeyboardPresentationController provides controller) {
        MainTabPager(
          state = rememberMainTabState().also { state = it },
          gestureAdmissionAllowed = true,
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) {
          Box(Modifier.fillMaxSize())
        }
      }
    }
    waitForIdle()

    runOnIdle { state.selectTab(Tab.Space) }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null && state.settledTab == Tab.Space }
    waitUntil { keyboardDriver.endpoints == listOf(SoftwareKeyboardPresentationEndpoint.Hidden) }
    runOnIdle { assertEquals(Tab.Home, state.focusEnabledTab) }

    runOnIdle { state.selectTab(Tab.Notes) }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null && state.settledTab == Tab.Notes }
    runOnIdle {
      assertEquals(Tab.Home, state.focusEnabledTab)
      assertEquals(listOf(SoftwareKeyboardPresentationEndpoint.Hidden), keyboardDriver.endpoints)
    }

    runOnIdle { keyboardDriver.acceptEndpoint() }
    waitUntil { state.focusEnabledTab != Tab.Home }
    runOnIdle { assertEquals(Tab.Notes, state.focusEnabledTab) }
  }

  @Test
  fun `cancelled tab swipe restores the keyboard without clearing focus`() = runComposeUiTest {
    lateinit var state: MainTabState
    val focusRequester = FocusRequester()
    val keyboardDriver = RecordingMainTabKeyboardDriver()
    val controller = SoftwareKeyboardPresentationController { keyboardDriver }
    var inputFocused = false

    setContent {
      CompositionLocalProvider(LocalSoftwareKeyboardPresentationController provides controller) {
        Box(Modifier.size(width = 320.dp, height = 640.dp)) {
          MainTabPager(
            state = rememberMainTabState().also { state = it },
            gestureAdmissionAllowed = true,
            modifier = Modifier.fillMaxSize().testTag(PagerTag),
          ) {
            Box(Modifier.fillMaxSize())
          }
          BasicTextField(
            value = "",
            onValueChange = {},
            modifier =
              Modifier.size(1.dp).focusRequester(focusRequester).onFocusChanged {
                inputFocused = it.isFocused
              },
          )
          LaunchedEffect(Unit) { focusRequester.requestFocus() }
        }
      }
    }
    waitUntil { inputFocused }

    onNodeWithTag(PagerTag).performTouchInput {
      down(center)
      moveBy(Offset(x = -80f, y = 0f), delayMillis = 100L)
    }
    waitUntil { state.motion?.source == MainTabMotionSource.DirectDrag }
    runOnIdle {
      assertTrue(inputFocused)
      assertTrue(keyboardDriver.endpoints.isEmpty())
      assertTrue(keyboardDriver.progress.any { it > 0f })
    }
    onNodeWithTag(PagerTag).performTouchInput { up() }
    waitUntil(timeoutMillis = 5_000L) { state.motion == null }

    runOnIdle {
      assertEquals(Tab.Home, state.settledTab)
      assertTrue(inputFocused)
      assertEquals(listOf(SoftwareKeyboardPresentationEndpoint.Shown), keyboardDriver.endpoints)
    }
  }

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
    const val BypassOutgoingRouteTag = "bypass-outgoing-route"
    const val ChromeTag = "main-tab-fixed-chrome"
    const val ChildPagerTag = "main-tab-child-pager"
    const val PagerTag = "main-tab-pager"
    val NoOpNestedScrollConnection = object : NestedScrollConnection {}
  }
}

private class RecordingMainTabKeyboardDriver : SoftwareKeyboardPresentationDriver {
  val progress = mutableListOf<Float>()
  val endpoints = mutableListOf<SoftwareKeyboardPresentationEndpoint>()

  override fun updateHiddenProgress(progress: Float) {
    this.progress += progress
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    endpoints += endpoint
    onAccepted()
  }

  override fun dispose() = Unit
}

private class DeferredMainTabKeyboardDriver : SoftwareKeyboardPresentationDriver {
  val progress = mutableListOf<Float>()
  val endpoints = mutableListOf<SoftwareKeyboardPresentationEndpoint>()
  private var endpointAcceptance: (() -> Unit)? = null

  override fun updateHiddenProgress(progress: Float) {
    this.progress += progress
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    endpoints += endpoint
    endpointAcceptance = onAccepted
  }

  fun acceptEndpoint() {
    endpointAcceptance?.invoke()
    endpointAcceptance = null
  }

  override fun dispose() = Unit
}
