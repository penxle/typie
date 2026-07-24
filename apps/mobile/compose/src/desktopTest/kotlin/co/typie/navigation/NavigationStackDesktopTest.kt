package co.typie.navigation

import androidx.compose.foundation.MutatePriority
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.gestures.scrollable
import androidx.compose.foundation.gestures.scrollable2D
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.TouchInjectionScope
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.performTrackpadInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.route.Route
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.bottombar.LocalBottomBarState
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarState
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

@OptIn(ExperimentalTestApi::class)
class NavigationStackDesktopTest {
  @Test
  fun navigationForegroundCanSharePointerInputWithRouteSurface() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var surfaceDownCount = 0

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize()
            .pointerInput(Unit) {
              awaitEachGesture {
                awaitFirstDown(requireUnconsumed = false)
                surfaceDownCount += 1
              }
            }
            .testTag("surface-$route")
        )
        NavigationForeground(sharePointerInputWithSiblings = true) {
          Box(Modifier.fillMaxSize().testTag("foreground-$route"))
        }
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    onNodeWithTag("foreground-$editorRoute").performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertEquals(1, surfaceDownCount)
  }

  @Test
  fun navigationForegroundRendersLocallyWithoutHost() = runComposeUiTest {
    setContent { NavigationForeground { Box(Modifier.size(20.dp).testTag(ForegroundFallbackTag)) } }

    onNodeWithTag(ForegroundFallbackTag).assertExists()
  }

  @Test
  fun navigationForegroundPreservesRouteIdentityAndDeclarationContext() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    val coveringRoute = Route.Document("covering-document-id")
    val topBarState = TopBarState()
    val bottomBarState = BottomBarState()
    val navigateAway = CompletableDeferred<Unit>()
    val returnToEditor = CompletableDeferred<Unit>()
    val foregroundIdentities = mutableMapOf<Route, Any>()
    val foregroundAttachCounts = mutableMapOf<Route, Int>()
    val foregroundDisposeCounts = mutableMapOf<Route, Int>()
    val observedRoutes = mutableMapOf<Route, Route?>()
    val observedContexts = mutableMapOf<Route, String>()
    val observedTopBars = mutableMapOf<Route, TopBarState?>()
    val observedBottomBars = mutableMapOf<Route, BottomBarState?>()

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = topBarState,
        bottomBarState = bottomBarState,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        CompositionLocalProvider(LocalForegroundTestValue provides "context-$route") {
          Box(Modifier.fillMaxSize().testTag("surface-$route"))
          NavigationForeground {
            val identity = remember { Any() }
            foregroundIdentities.putIfAbsent(route, identity)
            observedRoutes[route] = LocalRoute.current
            observedContexts[route] = LocalForegroundTestValue.current
            observedTopBars[route] = LocalTopBarState.current
            observedBottomBars[route] = LocalBottomBarState.current
            DisposableEffect(identity) {
              foregroundAttachCounts[route] = foregroundAttachCounts.getOrElse(route) { 0 } + 1
              onDispose {
                foregroundDisposeCounts[route] = foregroundDisposeCounts.getOrElse(route) { 0 } + 1
              }
            }
            Box(Modifier.fillMaxSize().testTag("foreground-$route"))
          }
        }
      }
      LaunchedEffect(Unit) {
        navigator.navigate(editorRoute)
        navigateAway.await()
        navigator.navigate(coveringRoute)
        returnToEditor.await()
        navigator.pop()
      }
    }

    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }
    onAllNodes(hasTestTag("foreground-$editorRoute")).assertCountEquals(1)
    val editorForegroundIdentity = foregroundIdentities.getValue(editorRoute)
    assertEquals(editorRoute, observedRoutes[editorRoute])
    assertEquals("context-$editorRoute", observedContexts[editorRoute])
    assertTrue(observedTopBars[editorRoute] === topBarState)
    assertTrue(observedBottomBars[editorRoute] === bottomBarState)

    navigateAway.complete(Unit)
    waitUntil { navigator.current == coveringRoute && !navigator.isTransitioning }
    assertEquals(1, foregroundAttachCounts[editorRoute])
    assertEquals(0, foregroundDisposeCounts[editorRoute] ?: 0)
    assertEquals(null, observedTopBars[editorRoute])
    assertEquals(null, observedBottomBars[editorRoute])

    returnToEditor.complete(Unit)
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }
    onAllNodes(hasTestTag("foreground-$editorRoute")).assertCountEquals(1)
    assertTrue(foregroundIdentities[editorRoute] === editorForegroundIdentity)
    assertEquals(1, foregroundAttachCounts[editorRoute])
    assertEquals(0, foregroundDisposeCounts[editorRoute] ?: 0)
    assertTrue(observedTopBars[editorRoute] === topBarState)
    assertTrue(observedBottomBars[editorRoute] === bottomBarState)
  }

  @Test
  fun navigationForegroundMatchesSurfaceDuringDirectBackDrag() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var platformTouchSlop = 0f

    setContent {
      platformTouchSlop = LocalViewConfiguration.current.touchSlop
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(Modifier.fillMaxSize().testTag("surface-$route"))
        NavigationForeground { Box(Modifier.fillMaxSize().testTag("foreground-$route")) }
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    val surface = onNodeWithTag("surface-$editorRoute")
    val foreground = onNodeWithTag("foreground-$editorRoute")
    foreground.performTouchInput {
      down(center)
      moveBy(Offset(x = platformTouchSlop * 4f, y = 0f), delayMillis = 100L)
    }
    waitUntil { surface.fetchSemanticsNode().boundsInRoot.left > 1f }

    assertEquals(
      surface.fetchSemanticsNode().boundsInRoot.left,
      foreground.fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )

    foreground.performTouchInput { up() }
    waitUntil { !navigator.isTransitioning }
  }

  @Test
  fun topBarBackdropStaysFixedOverLiveTransitionSurfaceComposite() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    val topBarState = TopBarState().apply { animatedAlpha = 1f }
    var platformTouchSlop = 0f

    setContent {
      platformTouchSlop = LocalViewConfiguration.current.touchSlop
      NavigationStack(
        navigator = navigator,
        topBarState = topBarState,
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        PublishNavigationTopBarBackdropStyle(
          background = if (route == editorRoute) Color.Blue else Color.Red
        )
        ProvideTopBar()
        Box(Modifier.fillMaxSize().testTag("surface-$route"))
        NavigationForeground { Box(Modifier.fillMaxSize().testTag("foreground-$route")) }
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    val surface = onNodeWithTag("surface-$editorRoute")
    val foreground = onNodeWithTag("foreground-$editorRoute")
    foreground.performTouchInput {
      down(center)
      moveBy(Offset(x = platformTouchSlop * 4f, y = 0f), delayMillis = 100L)
    }
    waitUntil { surface.fetchSemanticsNode().boundsInRoot.left > 1f }

    assertEquals(
      0f,
      onNodeWithTag(NavigationTopBarBackdropTestTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
    assertEquals(
      0f,
      onNodeWithTag(NavigationSceneSurfaceCompositeTestTag).fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )
    onNodeWithTag("surface-${Route.Home}").assertExists()
    assertEquals(
      surface.fetchSemanticsNode().boundsInRoot.left,
      foreground.fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )

    foreground.performTouchInput { up() }
    waitUntil { !navigator.isTransitioning }
  }

  @Test
  fun foregroundScrollableChildOwnsDragBeforeFullAreaBackSwipe() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var platformTouchSlop = 0f
    var childConsumed = 0f

    setContent {
      platformTouchSlop = LocalViewConfiguration.current.touchSlop
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(Modifier.fillMaxSize().testTag("surface-$route"))
        NavigationForeground {
          val scrollableState = rememberScrollableState { delta ->
            childConsumed += delta
            delta
          }
          Box(
            Modifier.fillMaxSize()
              .testTag("foreground-$route")
              .scrollable(scrollableState, Orientation.Horizontal)
          )
        }
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }
    val foreground = onNodeWithTag("foreground-$editorRoute")

    foreground.performTouchInput {
      down(center)
      moveBy(Offset(x = platformTouchSlop * 4f, y = 0f))
    }
    waitForIdle()

    assertTrue(abs(childConsumed) > 0.5f)
    assertEquals(
      0f,
      onNodeWithTag("surface-$editorRoute").fetchSemanticsNode().boundsInRoot.left,
      absoluteTolerance = 0.5f,
    )

    foreground.performTouchInput { up() }
    waitForIdle()
    assertEquals(editorRoute, navigator.current)
  }

  @Test
  fun mainAreaTrackpadClickDragPopsOnDesktop() = assertTrackpadClickDragPops(startAtEdge = false)

  @Test fun edgeTrackpadClickDragPopsOnDesktop() = assertTrackpadClickDragPops(startAtEdge = true)

  @Test
  fun verticalScrollSessionDoesNotBecomeBackSwipeAfterTurningRight() =
    assertGestureDoesNotPop(
      scrollConsumer = { delta -> if (abs(delta.y) > abs(delta.x)) delta else Offset.Zero },
      gesture = {
        down(center)
        moveBy(Offset(x = 0f, y = -120f))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  @Test
  fun leftNoOpSessionDoesNotBecomeBackSwipeAfterTurningRight() =
    assertGestureDoesNotPop(
      scrollConsumer = { Offset.Zero },
      gesture = {
        down(center)
        moveBy(Offset(x = -80f, y = 0f))
        moveBy(Offset(x = 240f, y = 0f))
        up()
      },
    )

  @Test
  fun horizontalScrollSessionDoesNotBecomeBackSwipeAtStartEdge() {
    var consumedHorizontalDrag = false
    assertGestureDoesNotPop(
      scrollConsumer = { delta ->
        if (!consumedHorizontalDrag && delta.x > 0f) {
          consumedHorizontalDrag = true
          delta
        } else {
          Offset.Zero
        }
      },
      gesture = {
        down(center)
        moveBy(Offset(x = 80f, y = 0f))
        moveBy(Offset(x = 160f, y = 0f))
        up()
      },
    )
  }

  @Test
  fun horizontalScrollableChildOwnsDragBeforeFullAreaBackSwipe() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var platformTouchSlop = 0f
    var childConsumed = 0f

    setContent {
      platformTouchSlop = LocalViewConfiguration.current.touchSlop
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        val scrollableState = rememberScrollableState { delta ->
          childConsumed += delta
          delta
        }
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
            .scrollable(scrollableState, Orientation.Horizontal)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }
    val routeNode = onNodeWithTag(EditorRouteTag)

    routeNode.performTouchInput {
      down(center)
      moveBy(Offset(x = platformTouchSlop - 1f, y = 0f))
    }
    waitForIdle()
    assertEquals(0f, routeNode.fetchSemanticsNode().boundsInRoot.left, absoluteTolerance = 0.5f)

    routeNode.performTouchInput { moveBy(Offset(x = platformTouchSlop * 3f, y = 0f)) }
    waitForIdle()
    assertTrue(abs(childConsumed) > 0.5f)
    assertEquals(0f, routeNode.fetchSemanticsNode().boundsInRoot.left, absoluteTolerance = 0.5f)

    routeNode.performTouchInput { up() }
    waitForIdle()
    assertEquals(editorRoute, navigator.current)
  }

  @Test
  fun childThatReachesItsScrollBoundaryKeepsFullAreaBackSwipeOwnership() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var platformTouchSlop = 0f
    var childConsumed = 0f

    setContent {
      platformTouchSlop = LocalViewConfiguration.current.touchSlop
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
            .pointerInput(Unit) {
              awaitEachGesture {
                val down = awaitFirstDown(requireUnconsumed = false)
                var canScroll = true
                while (true) {
                  val change =
                    awaitPointerEvent(PointerEventPass.Main).changes.firstOrNull {
                      it.id == down.id
                    } ?: break
                  if (!change.pressed) break
                  if (canScroll && change.positionChange().x > 0f) {
                    childConsumed += change.positionChange().x
                    canScroll = false
                    change.consume()
                  }
                }
              }
            }
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }
    val routeNode = onNodeWithTag(EditorRouteTag)

    routeNode.performTouchInput {
      down(center)
      moveBy(Offset(x = platformTouchSlop * 2f, y = 0f))
    }
    waitForIdle()
    assertEquals(platformTouchSlop * 2f, childConsumed, absoluteTolerance = 0.5f)
    assertEquals(0f, routeNode.fetchSemanticsNode().boundsInRoot.left, absoluteTolerance = 0.5f)

    routeNode.performTouchInput { moveBy(Offset(x = platformTouchSlop * 2f, y = 0f)) }
    waitForIdle()
    assertEquals(platformTouchSlop * 2f, childConsumed, absoluteTolerance = 0.5f)
    assertEquals(0f, routeNode.fetchSemanticsNode().boundsInRoot.left, absoluteTolerance = 0.5f)

    routeNode.performTouchInput { up() }
    waitForIdle()
    assertEquals(editorRoute, navigator.current)
  }

  @Test
  fun verticalEdgeSessionDoesNotBecomeBackSwipeAfterTurningRight() =
    assertGestureDoesNotPop(
      scrollConsumer = { delta -> if (abs(delta.y) > abs(delta.x)) delta else Offset.Zero },
      gesture = {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = 0f, y = -120f))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  @Test
  fun rejectedSessionAllowsBackSwipeAfterPointerUp() =
    assertGesturesPop(
      scrollConsumer = { delta -> if (abs(delta.y) > abs(delta.x)) delta else Offset.Zero },
      firstGesture = {
        down(center)
        moveBy(Offset(x = 0f, y = -120f))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
      secondGesture = {
        down(center)
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  @Test
  fun edgeSwipeBypassesHorizontalScrollableChild() =
    assertGesturePops(
      scrollConsumer = { it },
      gesture = {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      },
    )

  @Test
  fun pageAndEdgePointersCannotResumeAsBackSwipe() =
    assertGestureDoesNotPop(
      scrollConsumer = { Offset.Zero },
      gesture = {
        down(pointerId = 0, position = center)
        down(pointerId = 1, position = Offset(x = 10f, y = center.y))
        up(pointerId = 0)
        moveBy(pointerId = 1, delta = Offset(x = 220f, y = 0f))
        up(pointerId = 1)
      },
    )

  @Test
  fun releaseDecisionUsesScreenNormalizedVelocityBeforeProgress() {
    assertEquals(true, shouldCommitNavigationPop(0.51f, 100f, 320f))
    assertEquals(false, shouldCommitNavigationPop(0.5f, 100f, 320f))
    assertEquals(true, shouldCommitNavigationPop(0.2f, 320f, 320f))
    assertEquals(false, shouldCommitNavigationPop(0.8f, -320f, 320f))
    assertEquals(false, shouldCommitNavigationPop(0.2f, 399f, 400f))
  }

  @Test
  fun edgeTouchThatInterruptsScrollingCannotPopUntilTheNextGesture() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    lateinit var scrollableState: Scrollable2DState
    lateinit var compositionScope: CoroutineScope

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        if (route == editorRoute) {
          compositionScope = rememberCoroutineScope()
          scrollableState = rememberScrollable2DState { Offset.Zero }
          val connection = LocalNavigationPopNestedScroll.current
          val owner = remember { Any() }
          DisposableEffect(connection, owner) {
            connection?.registerScrollInterruption(
              owner = owner,
              isScrollInProgress = { scrollableState.isScrollInProgress },
              interrupt = {
                compositionScope.launch(start = CoroutineStart.UNDISPATCHED) {
                  scrollableState.scroll(MutatePriority.UserInput) {}
                }
              },
            )
            onDispose { connection?.unregisterScrollInterruption(owner) }
          }
        }
        Box(
          Modifier.fillMaxSize().testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    val releaseScroll = CompletableDeferred<Unit>()
    var scrollCancelled = false
    val scrollJob =
      compositionScope.launch(start = CoroutineStart.UNDISPATCHED) {
        try {
          scrollableState.scroll(MutatePriority.Default) { releaseScroll.await() }
        } catch (_: CancellationException) {
          scrollCancelled = true
        }
      }
    waitUntil { scrollableState.isScrollInProgress }

    try {
      onNodeWithTag(EditorRouteTag).performTouchInput {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      }
      waitUntil { scrollCancelled }

      assertEquals(editorRoute, navigator.current)

      onNodeWithTag(EditorRouteTag).performTouchInput {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = 220f, y = 0f))
        up()
      }
      waitForIdle()

      assertEquals(Route.Home, navigator.current)
    } finally {
      releaseScroll.complete(Unit)
      scrollJob.cancel()
    }
  }

  @Test
  fun edgeBackSwipeDiscardsThreeTimesPlatformTouchSlop() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")
    var activationDistancePx = 0f

    setContent {
      activationDistancePx = LocalViewConfiguration.current.touchSlop * 3f
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize().testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }
    val routeNode = onNodeWithTag(EditorRouteTag)

    routeNode.performTouchInput {
      down(Offset(x = 10f, y = center.y))
      moveBy(Offset(x = activationDistancePx - 1f, y = 0f))
    }
    waitForIdle()
    assertTrue(abs(routeNode.fetchSemanticsNode().boundsInRoot.left) < 0.1f)

    routeNode.performTouchInput { moveBy(Offset(x = 1f, y = 0f)) }
    waitForIdle()
    assertTrue(abs(routeNode.fetchSemanticsNode().boundsInRoot.left) < 0.1f)

    routeNode.performTouchInput { moveBy(Offset(x = 5f, y = 0f)) }
    waitUntil { routeNode.fetchSemanticsNode().boundsInRoot.left > 4f }
    assertTrue(abs(routeNode.fetchSemanticsNode().boundsInRoot.left - 5f) < 0.5f)

    routeNode.performTouchInput { up() }
    waitForIdle()
    assertEquals(editorRoute, navigator.current)
  }

  @Test
  fun fastReverseEdgeReleaseCancelsEvenAfterHalfway() =
    assertGestureDoesNotPop(
      scrollConsumer = { Offset.Zero },
      gesture = {
        down(Offset(x = 10f, y = center.y))
        moveBy(Offset(x = 220f, y = 0f), delayMillis = 500L)
        repeat(8) { moveBy(Offset(x = -5f, y = 0f), delayMillis = 1L) }
        up()
      },
    )

  private fun assertGestureDoesNotPop(
    scrollConsumer: (Offset) -> Offset,
    gesture: TouchInjectionScope.() -> Unit,
  ) = assertGestureResult(scrollConsumer, shouldPop = false, gesture)

  private fun assertTrackpadClickDragPops(startAtEdge: Boolean) = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize().testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    onNodeWithTag(EditorRouteTag).performTrackpadInput {
      moveTo(if (startAtEdge) Offset(x = 10f, y = center.y) else center)
      press()
      repeat(11) { moveBy(Offset(x = 20f, y = 0f), delayMillis = 100L) }
    }
    waitUntil { onNodeWithTag(EditorRouteTag).fetchSemanticsNode().boundsInRoot.left > 100f }
    onNodeWithTag(EditorRouteTag).performTrackpadInput { release() }
    waitForIdle()

    assertEquals(Route.Home, navigator.current)
    onAllNodes(hasTestTag(EditorRouteTag)).assertCountEquals(0)
  }

  private fun assertGesturePops(
    scrollConsumer: (Offset) -> Offset,
    gesture: TouchInjectionScope.() -> Unit,
  ) = assertGestureResult(scrollConsumer, shouldPop = true, gesture)

  private fun assertGesturesPop(
    scrollConsumer: (Offset) -> Offset,
    firstGesture: TouchInjectionScope.() -> Unit,
    secondGesture: TouchInjectionScope.() -> Unit,
  ) = assertGestureResult(scrollConsumer, shouldPop = true, firstGesture, secondGesture)

  private fun assertGestureResult(
    scrollConsumer: (Offset) -> Offset,
    shouldPop: Boolean,
    vararg gestures: TouchInjectionScope.() -> Unit,
  ) = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val editorRoute = Route.Editor("document-id")

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        val scrollableState = rememberScrollable2DState(scrollConsumer)
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == editorRoute) EditorRouteTag else HomeRouteTag)
            .navigationPopNestedScroll()
            .scrollable2D(scrollableState)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(editorRoute) }
    }
    waitUntil { navigator.current == editorRoute && !navigator.isTransitioning }

    gestures.forEach { gesture ->
      onNodeWithTag(EditorRouteTag).performTouchInput(gesture)
      waitForIdle()
    }

    assertEquals(if (shouldPop) Route.Home else editorRoute, navigator.current)
  }

  private companion object {
    const val EditorRouteTag = "editor-route"
    const val ForegroundFallbackTag = "foreground-fallback"
    const val HomeRouteTag = "home-route"
  }
}

private val LocalForegroundTestValue = staticCompositionLocalOf { "missing" }
