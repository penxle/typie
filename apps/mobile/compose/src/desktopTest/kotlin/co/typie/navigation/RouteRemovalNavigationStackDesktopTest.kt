package co.typie.navigation

import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.gestures.scrollable2D
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.route.Route
import co.typie.ui.component.topbar.TopBarState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineExceptionHandler

@OptIn(ExperimentalTestApi::class)
class RouteRemovalNavigationStackDesktopTest {
  @Test
  fun cancelRemovalStopsMultiPopAfterGuardedRouteBecomesCurrent() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    val topRoute = Route.Document("top")
    val popRequested = CompletableDeferred<Unit>()
    var currentAtDecision: Route? = null
    var rollbackCount = 0
    var popResult: NavigationResult? = null

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation =
          RouteRemovalPreparation.NeedsDecision

        override suspend fun resolveDecision(): RouteRemovalDecision {
          currentAtDecision = navigator.current
          return RouteRemovalDecision.CancelRemoval
        }

        override suspend fun rollback() {
          rollbackCount++
        }
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
      LaunchedEffect(Unit) {
        navigator.navigate(guardedRoute)
        navigator.navigate(topRoute)
        popRequested.await()
        popResult = navigator.popTo(Route.Home)
      }
    }
    waitUntil { navigator.current == topRoute && !navigator.isTransitioning }

    popRequested.complete(Unit)
    waitUntil { popResult != null }

    assertEquals(guardedRoute, currentAtDecision)
    assertEquals(NavigationResult.StoppedAt(guardedRoute), popResult)
    assertEquals(listOf(Route.Home, guardedRoute), navigator.stack)
    assertEquals(1, rollbackCount)
  }

  @Test
  fun replacedInterceptorDuringPreparationRestartsRemoval() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    val popRequested = CompletableDeferred<Unit>()
    val preparationStarted = CompletableDeferred<Unit>()
    val releasePreparation = CompletableDeferred<Unit>()
    var originalRollbackCount = 0
    var replacementPrepareCount = 0
    var popResult: Result<NavigationResult>? = null

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          preparationStarted.complete(Unit)
          releasePreparation.await()
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          error("Ready preparation must not require a decision")

        override suspend fun rollback() {
          originalRollbackCount++
        }
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize())
      }
      LaunchedEffect(Unit) {
        navigator.navigate(guardedRoute)
        popRequested.await()
        popResult = runCatching { navigator.pop() }
      }
    }
    waitUntil { navigator.current == guardedRoute && !navigator.isTransitioning }

    popRequested.complete(Unit)
    waitUntil { preparationStarted.isCompleted }
    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          replacementPrepareCount++
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          error("Ready preparation must not require a decision")

        override suspend fun rollback() = Unit
      },
    )
    releasePreparation.complete(Unit)
    waitUntil { popResult != null }

    assertEquals(NavigationResult.ReachedTarget, popResult?.getOrThrow())
    assertEquals(Route.Home, navigator.current)
    assertEquals(1, originalRollbackCount)
    assertEquals(1, replacementPrepareCount)
  }

  @Test
  fun backSwipeUsesRouteRemovalInterceptor() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    var rollbackCount = 0

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation =
          RouteRemovalPreparation.NeedsDecision

        override suspend fun resolveDecision(): RouteRemovalDecision =
          RouteRemovalDecision.CancelRemoval

        override suspend fun rollback() {
          rollbackCount++
        }
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        val scrollableState = rememberScrollable2DState { Offset.Zero }
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == guardedRoute) GuardedRouteTag else HomeRouteTag)
            .navigationPopNestedScroll()
            .scrollable2D(scrollableState)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(guardedRoute) }
    }
    waitUntil { navigator.current == guardedRoute && !navigator.isTransitioning }

    onNodeWithTag(GuardedRouteTag).performTouchInput {
      down(center)
      moveBy(Offset(x = 220f, y = 0f))
      up()
    }
    waitUntil { rollbackCount == 1 && !navigator.isTransitioning }

    assertEquals(guardedRoute, navigator.current)
    assertEquals(listOf(Route.Home, guardedRoute), navigator.stack)
  }

  @Test
  fun mainAreaBackSwipeDoesNotMoveWhileRemovalIsPreparing() =
    assertBackSwipeDoesNotMoveWhileRemovalIsPreparing(startAtEdge = false)

  @Test
  fun edgeBackSwipeDoesNotMoveWhileRemovalIsPreparing() =
    assertBackSwipeDoesNotMoveWhileRemovalIsPreparing(startAtEdge = true)

  @Test
  fun committedBackSwipeContinuesFromReleasedPosition() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val route = Route.Folder("unguarded")

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) {
        Box(Modifier.fillMaxSize().testTag(if (it == route) UnguardedRouteTag else HomeRouteTag))
      }
      LaunchedEffect(Unit) { navigator.navigate(route) }
    }
    waitUntil { navigator.current == route && !navigator.isTransitioning }

    val clock = mainClock
    val routeNode = onNodeWithTag(UnguardedRouteTag)
    routeNode.performTouchInput {
      down(center)
      moveBy(Offset(x = 100f, y = 0f))
      moveBy(Offset(x = 120f, y = 0f))
      clock.autoAdvance = false
      up()
    }
    clock.advanceTimeByFrame()
    val releasedLeft = routeNode.fetchSemanticsNode().boundsInRoot.left
    assertTrue(
      releasedLeft > 150f,
      "A committed back swipe must retain its released position: released=$releasedLeft",
    )

    clock.advanceTimeBy(100L)
    val animatedLeft = routeNode.fetchSemanticsNode().boundsInRoot.left

    assertTrue(
      animatedLeft > releasedLeft,
      "A committed back swipe must continue toward the exit: released=$releasedLeft, animated=$animatedLeft",
    )
    clock.autoAdvance = true
    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
  }

  @Test
  fun committedBackSwipeContinuesWhileRemovalIsPreparing() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    val preparationStarted = CompletableDeferred<Unit>()
    val releasePreparation = CompletableDeferred<Unit>()

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          preparationStarted.complete(Unit)
          releasePreparation.await()
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          error("Ready preparation must not require a decision")

        override suspend fun rollback() = Unit
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == guardedRoute) GuardedRouteTag else HomeRouteTag)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(guardedRoute) }
    }
    waitUntil { navigator.current == guardedRoute && !navigator.isTransitioning }

    val clock = mainClock
    val routeNode = onNodeWithTag(GuardedRouteTag)
    routeNode.performTouchInput {
      down(center)
      moveBy(Offset(x = 100f, y = 0f))
      moveBy(Offset(x = 120f, y = 0f))
      clock.autoAdvance = false
      up()
    }
    clock.advanceTimeByFrame()
    val releasedLeft = routeNode.fetchSemanticsNode().boundsInRoot.left
    clock.advanceTimeBy(100L)
    val animatedLeft = routeNode.fetchSemanticsNode().boundsInRoot.left

    try {
      assertTrue(preparationStarted.isCompleted)
      assertEquals(guardedRoute, navigator.current)
      assertTrue(
        animatedLeft > releasedLeft,
        "A committed back swipe must continue while removal prepares: " +
          "released=$releasedLeft, animated=$animatedLeft",
      )
    } finally {
      releasePreparation.complete(Unit)
      clock.autoAdvance = true
    }
    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
  }

  @Test
  fun delayedCommittedBackSwipeReturnsToRouteThenAutomaticallyPops() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    val delayedRouteSettled = CompletableDeferred<Unit>()
    val releasePreparation = CompletableDeferred<Unit>()

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          checkNotNull(onDelayed).invoke()
          delayedRouteSettled.complete(Unit)
          releasePreparation.await()
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          error("Ready preparation must not require a decision")

        override suspend fun rollback() = Unit
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == guardedRoute) GuardedRouteTag else HomeRouteTag)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(guardedRoute) }
    }
    waitUntil { navigator.current == guardedRoute && !navigator.isTransitioning }

    val routeNode = onNodeWithTag(GuardedRouteTag)
    routeNode.performTouchInput {
      down(center)
      moveBy(Offset(x = 100f, y = 0f))
      moveBy(Offset(x = 120f, y = 0f))
      up()
    }
    waitUntil { delayedRouteSettled.isCompleted }

    assertEquals(guardedRoute, navigator.current)
    assertEquals(0f, routeNode.fetchSemanticsNode().boundsInRoot.left)

    releasePreparation.complete(Unit)
    waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
  }

  @Test
  fun slowHiddenRouteBecomesCurrentThenMultiPopAutomaticallyContinues() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    val topRoute = Route.Document("top")
    val requestRemoval = CompletableDeferred<Unit>()
    val delayedRouteSettled = CompletableDeferred<Unit>()
    val releasePreparation = CompletableDeferred<Unit>()
    var removalResult: NavigationResult? = null

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          checkNotNull(onDelayed).invoke()
          delayedRouteSettled.complete(Unit)
          releasePreparation.await()
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          error("Ready preparation must not require a decision")

        override suspend fun rollback() = Unit
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(Modifier.fillMaxSize().testTag(route.toString()))
      }
      LaunchedEffect(Unit) {
        navigator.navigate(guardedRoute)
        navigator.navigate(topRoute)
        requestRemoval.await()
        removalResult = navigator.popTo(Route.Home)
      }
    }
    waitUntil { navigator.current == topRoute && !navigator.isTransitioning }

    requestRemoval.complete(Unit)
    waitUntil { delayedRouteSettled.isCompleted }

    assertEquals(guardedRoute, navigator.current)
    assertEquals(listOf(Route.Home, guardedRoute), navigator.stack)
    assertEquals(null, removalResult)

    releasePreparation.complete(Unit)
    waitUntil { navigator.current == Route.Home && removalResult != null }
    assertEquals(NavigationResult.ReachedTarget, removalResult)
  }

  @Test
  fun failedRemovalRestoresCommittedBackSwipeBeforeRequestingDecision() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val guardedRoute = Route.Folder("guarded")
    val preparationStarted = CompletableDeferred<Unit>()
    val preparationResult = CompletableDeferred<RouteRemovalPreparation>()
    val decisionStarted = CompletableDeferred<Unit>()
    val releaseDecision = CompletableDeferred<Unit>()
    var homeClicks = 0

    navigator.routeRemovals.register(
      guardedRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          preparationStarted.complete(Unit)
          return preparationResult.await()
        }

        override suspend fun resolveDecision(): RouteRemovalDecision {
          decisionStarted.complete(Unit)
          releaseDecision.await()
          return RouteRemovalDecision.CancelRemoval
        }

        override suspend fun rollback() = Unit
      },
    )

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize()
            .testTag(if (route == guardedRoute) GuardedRouteTag else HomeRouteTag)
            .then(if (route == Route.Home) Modifier.clickable { homeClicks++ } else Modifier)
        )
      }
      LaunchedEffect(Unit) { navigator.navigate(guardedRoute) }
    }
    waitUntil { navigator.current == guardedRoute && !navigator.isTransitioning }

    val clock = mainClock
    val routeNode = onNodeWithTag(GuardedRouteTag)
    try {
      routeNode.performTouchInput {
        down(center)
        moveBy(Offset(x = 100f, y = 0f))
        moveBy(Offset(x = 120f, y = 0f))
        clock.autoAdvance = false
        up()
      }
      clock.advanceTimeBy(2_000L)

      assertTrue(preparationStarted.isCompleted)
      assertEquals(guardedRoute, navigator.current)
      onNodeWithTag(HomeRouteTag).performTouchInput { click(center) }
      assertEquals(0, homeClicks)

      preparationResult.complete(RouteRemovalPreparation.NeedsDecision)
      clock.autoAdvance = true
      waitUntil { decisionStarted.isCompleted }

      assertEquals(guardedRoute, navigator.current)
      assertEquals(0f, routeNode.fetchSemanticsNode().boundsInRoot.left)
    } finally {
      preparationResult.complete(RouteRemovalPreparation.NeedsDecision)
      releaseDecision.complete(Unit)
      clock.autoAdvance = true
    }

    waitUntil { !navigator.isTransitioning }
    assertEquals(listOf(Route.Home, guardedRoute), navigator.stack)
  }

  @Test
  fun differentTargetRemovalWaitsForCommittedBackSwipeToSettle() = runComposeUiTest {
    val navigator = Navigator(Route.Home)
    val middleRoute = Route.Folder("middle")
    val topRoute = Route.Document("top")
    val requestRemoval = CompletableDeferred<Unit>()
    var removalResult: Result<NavigationResult>? = null

    setContent {
      NavigationStack(
        navigator = navigator,
        topBarState = remember { TopBarState() },
        modifier = Modifier.size(width = 320.dp, height = 640.dp),
      ) { route ->
        Box(
          Modifier.fillMaxSize().testTag(if (route == topRoute) TopRouteTag else BackgroundRouteTag)
        )
      }
      LaunchedEffect(Unit) {
        navigator.navigate(middleRoute)
        navigator.navigate(topRoute)
        requestRemoval.await()
        removalResult = runCatching { navigator.popTo(Route.Home) }
      }
    }
    waitUntil { navigator.current == topRoute && !navigator.isTransitioning }

    val routeNode = onNodeWithTag(TopRouteTag)
    routeNode.performTouchInput {
      down(center)
      moveBy(Offset(x = 100f, y = 0f))
      moveBy(Offset(x = 120f, y = 0f))
    }
    requestRemoval.complete(Unit)
    waitUntil { navigator.popRequested }
    routeNode.performTouchInput { up() }
    waitUntil { removalResult != null }

    assertEquals(NavigationResult.ReachedTarget, removalResult?.getOrThrow())
    assertEquals(Route.Home, navigator.current)
    assertFalse(navigator.isTransitioning)
  }

  @Test
  fun rollbackFailureStillCompletesRemovalTransition() {
    val uncaughtFailure = CompletableDeferred<Throwable>()
    runComposeUiTest(
      effectContext =
        CoroutineExceptionHandler { _, throwable -> uncaughtFailure.complete(throwable) }
    ) {
      val navigator = Navigator(Route.Home)
      val guardedRoute = Route.Folder("guarded")
      val popRequested = CompletableDeferred<Unit>()
      val decisionFailure = IllegalStateException("decision failure")
      val rollbackFailure = IllegalStateException("rollback failure")
      var rollbackCount = 0
      var popResult: Result<NavigationResult>? = null

      navigator.routeRemovals.register(
        guardedRoute,
        object : RouteRemovalInterceptor {
          override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation =
            RouteRemovalPreparation.NeedsDecision

          override suspend fun resolveDecision(): RouteRemovalDecision = throw decisionFailure

          override suspend fun rollback() {
            rollbackCount++
            throw rollbackFailure
          }
        },
      )

      setContent {
        NavigationStack(
          navigator = navigator,
          topBarState = remember { TopBarState() },
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) {
          Box(Modifier.fillMaxSize())
        }
        LaunchedEffect(Unit) {
          navigator.navigate(guardedRoute)
          popRequested.await()
          popResult = runCatching { navigator.pop() }
        }
      }
      waitUntil { navigator.current == guardedRoute && !navigator.isTransitioning }

      popRequested.complete(Unit)
      waitUntil { rollbackCount == 1 }
      waitForIdle()

      assertFalse(navigator.isTransitioning)
      val failure = assertNotNull(popResult).exceptionOrNull()
      assertEquals(decisionFailure::class, failure?.let { it::class })
      assertEquals(decisionFailure.message, failure?.message)
      assertEquals(listOf(rollbackFailure), decisionFailure.suppressed.toList())
      assertFalse(uncaughtFailure.isCompleted)
    }
  }

  private fun assertBackSwipeDoesNotMoveWhileRemovalIsPreparing(startAtEdge: Boolean) =
    runComposeUiTest {
      val navigator = Navigator(Route.Home)
      val route = Route.Folder("preparing")
      val popRequested = CompletableDeferred<Unit>()
      val preparationStarted = CompletableDeferred<Unit>()
      val releasePreparation = CompletableDeferred<Unit>()

      navigator.routeRemovals.register(
        route,
        object : RouteRemovalInterceptor {
          override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
            preparationStarted.complete(Unit)
            releasePreparation.await()
            return RouteRemovalPreparation.Ready
          }

          override suspend fun resolveDecision(): RouteRemovalDecision =
            error("Ready preparation must not require a decision")

          override suspend fun rollback() = Unit
        },
      )

      setContent {
        NavigationStack(
          navigator = navigator,
          topBarState = remember { TopBarState() },
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        ) {
          Box(Modifier.fillMaxSize().testTag(if (it == route) PreparingRouteTag else HomeRouteTag))
        }
        LaunchedEffect(Unit) {
          navigator.navigate(route)
          popRequested.await()
          navigator.pop()
        }
      }
      waitUntil { navigator.current == route && !navigator.isTransitioning }

      val routeNode = onNodeWithTag(PreparingRouteTag)
      val settledLeft = routeNode.fetchSemanticsNode().boundsInRoot.left
      popRequested.complete(Unit)
      waitUntil { preparationStarted.isCompleted }

      val draggedLeft =
        try {
          routeNode.performTouchInput {
            down(if (startAtEdge) Offset(x = 10f, y = center.y) else center)
            moveBy(Offset(x = 100f, y = 0f))
            moveBy(Offset(x = 120f, y = 0f))
          }
          mainClock.advanceTimeByFrame()
          routeNode.fetchSemanticsNode().boundsInRoot.left
        } finally {
          routeNode.performTouchInput { up() }
          releasePreparation.complete(Unit)
        }

      waitUntil { navigator.current == Route.Home && !navigator.isTransitioning }
      assertEquals(settledLeft, draggedLeft)
    }

  private companion object {
    const val GuardedRouteTag = "guarded-route"
    const val HomeRouteTag = "home-route"
    const val UnguardedRouteTag = "unguarded-route"
    const val TopRouteTag = "top-route"
    const val BackgroundRouteTag = "background-route"
    const val PreparingRouteTag = "preparing-route"
  }
}
