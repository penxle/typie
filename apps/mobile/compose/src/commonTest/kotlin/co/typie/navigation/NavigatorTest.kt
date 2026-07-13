package co.typie.navigation

import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotSame
import kotlin.test.assertNull
import kotlin.test.assertTrue
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
