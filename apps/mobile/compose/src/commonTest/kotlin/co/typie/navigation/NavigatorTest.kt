package co.typie.navigation

import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNotSame
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class NavigatorTest {

  @Test
  fun initialState() {
    val nav = Navigator(Route.Home)
    assertEquals(Route.Home, nav.current)
    assertNull(nav.previous)
    assertFalse(nav.canPop)
    assertEquals(1, nav.stack.size)
  }

  @Test
  fun navigate() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Folder("1"))
    assertEquals(Route.Folder("1"), nav.current)
    assertEquals(Route.Home, nav.previous)
    assertTrue(nav.canPop)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun duplicateNavigateWaitsForTheActivePush() = runTest {
    val nav = Navigator(Route.Home)
    val folderRoute = Route.Folder("1")
    val first = async { nav.navigate(folderRoute) }
    advanceUntilIdle()
    val second = async { nav.navigate(folderRoute) }
    advanceUntilIdle()

    assertFalse(first.isCompleted)
    assertFalse(second.isCompleted)
    nav.completeTransition()

    assertEquals(NavigationResult.ReachedTarget, first.await())
    assertEquals(NavigationResult.ReachedTarget, second.await())
  }

  @Test
  fun differentNavigateDuringActivePushIsNotStarted() = runTest {
    val nav = Navigator(Route.Home)
    val push = async { nav.navigate(Route.Folder("1")) }
    advanceUntilIdle()

    val result = nav.navigate(Route.Space)
    nav.completeTransition()
    push.await()

    assertEquals(NavigationResult.NotStarted, result)
  }

  @Test
  fun pop() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Folder("1"))
    popAndComplete(nav)
    assertEquals(Route.Home, nav.current)
    assertFalse(nav.canPop)
  }

  @Test
  fun popAtRoot() = runTest {
    val nav = Navigator(Route.Home)
    assertEquals(NavigationResult.ReachedTarget, nav.pop())
    assertEquals(Route.Home, nav.current)
  }

  @Test
  fun popReturnsStoppedAtFromRemovalOperation() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    navigateAndComplete(nav, editorRoute)
    val result = async { nav.pop() }
    advanceUntilIdle()

    nav.consumePopRequest()
    nav.completeTransition(result = NavigationResult.StoppedAt(editorRoute))

    assertEquals(NavigationResult.StoppedAt(editorRoute), result.await())
  }

  @Test
  fun overlappingPopWaitsForActiveRemovalResult() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    navigateAndComplete(nav, editorRoute)
    val first = async { nav.pop() }
    advanceUntilIdle()
    val second = async { nav.pop() }
    advanceUntilIdle()

    assertFalse(first.isCompleted)
    assertFalse(second.isCompleted)
    nav.consumePopRequest()
    nav.completeTransition(result = NavigationResult.StoppedAt(editorRoute))

    assertEquals(NavigationResult.StoppedAt(editorRoute), first.await())
    assertEquals(NavigationResult.StoppedAt(editorRoute), second.await())
  }

  @Test
  fun navigateToCurrentDuringRemovalDoesNotReportAStableOutcome() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    navigateAndComplete(nav, editorRoute)
    val removal = async { nav.pop() }
    advanceUntilIdle()

    val result = nav.navigate(editorRoute)
    nav.consumePopRequest()
    nav.completeTransition()
    removal.await()

    assertEquals(NavigationResult.NotStarted, result)
  }

  @Test
  fun differentRemovalDuringActiveRemovalDoesNotReportAStableStop() = runTest {
    val nav = Navigator(Route.Home)
    val spaceRoute = Route.Space
    val folderRoute = Route.Folder("1")
    navigateAndComplete(nav, spaceRoute)
    navigateAndComplete(nav, folderRoute)
    val removal = async { nav.popTo(Route.Home) }
    advanceUntilIdle()

    val result = nav.pop()
    nav.consumePopRequest()
    nav.completeTransition()
    removal.await()

    assertEquals(NavigationResult.NotStarted, result)
  }

  @Test
  fun popTo() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Space)
    navigateAndComplete(nav, Route.Folder("1"))
    navigateAndComplete(nav, Route.Folder("2"))
    popToAndComplete(nav, Route.Space)
    assertEquals(Route.Space, nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToNotInStack() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Folder("1"))
    assertEquals(NavigationResult.NotStarted, nav.popTo(Route.Notes))
    assertEquals(Route.Folder("1"), nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToRoot() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Space)
    navigateAndComplete(nav, Route.Folder("1"))
    val job = launch { nav.popToRoot() }
    advanceUntilIdle()
    nav.performPopTo(Route.Home)
    nav.consumePopRequest()
    nav.completeTransition()
    job.join()
    assertEquals(Route.Home, nav.current)
    assertEquals(1, nav.stack.size)
  }

  @Test
  fun lastOperationOnNavigate() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Folder("1"))
    assertEquals(NavOperation.Push, nav.lastOperation)
  }

  @Test
  fun lastOperationOnPop() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Folder("1"))
    popAndComplete(nav)
    assertEquals(NavOperation.Pop, nav.lastOperation)
  }

  @Test
  fun clearDropsViewModelStores() {
    val nav = Navigator(Route.Home)
    val originalStore = nav.viewModelStoreFor(Route.Home)

    nav.clear()

    val recreatedStore = nav.viewModelStoreFor(Route.Home)
    assertNotSame(originalStore, recreatedStore)
  }

  @Test
  fun preparedAdjacentRemovalCommitsExactDocumentEditorPair() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    nav.routeRemovals.register(
      editorRoute,
      RecordingNavigatorInterceptor(RouteRemovalPreparation.Ready),
    )

    val prepared = assertNotNull(nav.prepareAdjacentRemoval(documentRoute, editorRoute))

    assertTrue(nav.isTransitioning)
    val commit = async { nav.commitAdjacentRemoval(prepared) }
    advanceUntilIdle()

    assertEquals(documentRoute, nav.current)
    assertTrue(nav.popRequested)
    assertEquals(RouteRemovalPolicy.BypassInterceptors, nav.peekRemovalPolicy())
    val target = assertNotNull(nav.peekPopTarget())
    val removedRoutes = nav.performPopTo(target)
    nav.consumePopRequest()
    nav.completeTransition()

    assertEquals(NavigationResult.ReachedTarget, commit.await())
    assertEquals(listOf(documentRoute, editorRoute), removedRoutes)
    assertEquals(Route.Home, nav.current)
    assertFalse(nav.isTransitioning)
  }

  @Test
  fun adjacentRemovalDoesNotProvideDelayedPresentationCallback() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    var receivedDelayedCallback = true
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    nav.routeRemovals.register(
      editorRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          receivedDelayedCallback = onDelayed != null
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          RouteRemovalDecision.CancelRemoval

        override suspend fun rollback() = Unit
      },
    )

    val prepared = assertNotNull(nav.prepareAdjacentRemoval(documentRoute, editorRoute))

    assertFalse(receivedDelayedCallback)
    nav.rollbackAdjacentRemoval(prepared)
  }

  @Test
  fun navigateToCurrentDuringAdjacentRemovalPreparationIsNotStarted() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    nav.routeRemovals.register(
      editorRoute,
      RecordingNavigatorInterceptor(RouteRemovalPreparation.Ready),
    )
    val prepared = assertNotNull(nav.prepareAdjacentRemoval(documentRoute, editorRoute))

    assertEquals(NavigationResult.NotStarted, nav.navigate(documentRoute))

    nav.rollbackAdjacentRemoval(prepared)
  }

  @Test
  fun adjacentRemovalRollbackKeepsPairAndReleasesNavigationLease() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    val interceptor = RecordingNavigatorInterceptor(RouteRemovalPreparation.Ready)
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    nav.routeRemovals.register(editorRoute, interceptor)
    val prepared = assertNotNull(nav.prepareAdjacentRemoval(documentRoute, editorRoute))

    nav.rollbackAdjacentRemoval(prepared)

    assertEquals(documentRoute, nav.current)
    assertEquals(editorRoute, nav.previous)
    assertEquals(1, interceptor.rollbacks)
    assertFalse(nav.isTransitioning)
    navigateAndComplete(nav, Route.Space)
    assertEquals(Route.Space, nav.current)
  }

  @Test
  fun adjacentRemovalStillCommitsAfterEditorInterceptorReplacement() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    nav.routeRemovals.register(
      editorRoute,
      RecordingNavigatorInterceptor(RouteRemovalPreparation.Ready),
    )
    val prepared = assertNotNull(nav.prepareAdjacentRemoval(documentRoute, editorRoute))

    nav.routeRemovals.register(
      editorRoute,
      RecordingNavigatorInterceptor(RouteRemovalPreparation.Ready),
    )
    val commit = async { nav.commitAdjacentRemoval(prepared) }
    advanceUntilIdle()

    assertEquals(documentRoute, nav.current)
    val target = assertNotNull(nav.peekPopTarget())
    nav.performPopTo(target)
    nav.consumePopRequest()
    nav.completeTransition()

    assertEquals(NavigationResult.ReachedTarget, commit.await())
    assertEquals(Route.Home, nav.current)
    assertFalse(nav.isTransitioning)
  }

  @Test
  fun adjacentRemovalPreparationFailsIfInterceptorIsReplacedWhilePreparing() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    val preparationStarted = CompletableDeferred<Unit>()
    val releasePreparation = CompletableDeferred<Unit>()
    var rollbacks = 0
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    nav.routeRemovals.register(
      editorRoute,
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
          preparationStarted.complete(Unit)
          releasePreparation.await()
          return RouteRemovalPreparation.Ready
        }

        override suspend fun resolveDecision(): RouteRemovalDecision =
          RouteRemovalDecision.CancelRemoval

        override suspend fun rollback() {
          rollbacks++
        }
      },
    )

    val preparation = async { nav.prepareAdjacentRemoval(documentRoute, editorRoute) }
    advanceUntilIdle()
    assertTrue(preparationStarted.isCompleted)
    nav.routeRemovals.register(
      editorRoute,
      RecordingNavigatorInterceptor(RouteRemovalPreparation.Ready),
    )
    releasePreparation.complete(Unit)
    advanceUntilIdle()

    assertNull(preparation.await())
    assertEquals(1, rollbacks)
    assertFalse(nav.isTransitioning)
  }

  @Test
  fun failedAdjacentPreparationKeepsBothRoutes() = runTest {
    val nav = Navigator(Route.Home)
    val editorRoute = Route.Editor("1")
    val documentRoute = Route.Document("1")
    navigateAndComplete(nav, editorRoute)
    navigateAndComplete(nav, documentRoute)
    val interceptor = RecordingNavigatorInterceptor(RouteRemovalPreparation.NeedsDecision)
    nav.routeRemovals.register(editorRoute, interceptor)

    assertNull(nav.prepareAdjacentRemoval(documentRoute, editorRoute))
    assertEquals(documentRoute, nav.current)
    assertEquals(editorRoute, nav.previous)
    assertEquals(1, interceptor.rollbacks)
    assertFalse(nav.isTransitioning)
  }

  context(testScope: TestScope)
  private suspend fun navigateAndComplete(nav: Navigator, route: Route) {
    with(testScope) {
      val job = launch { nav.navigate(route) }
      advanceUntilIdle()
      nav.completeTransition()
      advanceUntilIdle()
      job.join()
    }
  }

  context(testScope: TestScope)
  private suspend fun popAndComplete(nav: Navigator) {
    with(testScope) {
      val job = launch { nav.pop() }
      advanceUntilIdle()
      if (nav.popRequested) {
        nav.previous?.let(nav::performPopTo)
        nav.consumePopRequest()
      }
      nav.completeTransition()
      advanceUntilIdle()
      job.join()
    }
  }

  context(testScope: TestScope)
  private suspend fun popToAndComplete(nav: Navigator, route: Route) {
    with(testScope) {
      val job = launch { nav.popTo(route) }
      advanceUntilIdle()
      val popTarget = nav.peekPopTarget()
      if (nav.popRequested && popTarget != null) {
        nav.performPopTo(popTarget)
        nav.consumePopRequest()
      }
      nav.completeTransition()
      advanceUntilIdle()
      job.join()
    }
  }
}

private class RecordingNavigatorInterceptor(private val preparation: RouteRemovalPreparation) :
  RouteRemovalInterceptor {
  var rollbacks = 0

  override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation =
    preparation

  override suspend fun resolveDecision(): RouteRemovalDecision = RouteRemovalDecision.CancelRemoval

  override suspend fun rollback() {
    rollbacks++
  }
}
