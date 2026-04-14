package co.typie.navigation

import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotSame
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
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
    nav.pop()
    assertEquals(Route.Home, nav.current)
  }

  @Test
  fun popTo() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Space)
    navigateAndComplete(nav, Route.Folder("1"))
    navigateAndComplete(nav, Route.Folder("2"))
    nav.popTo(Route.Space)
    assertEquals(Route.Space, nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToNotInStack() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Folder("1"))
    nav.popTo(Route.Notes)
    assertEquals(Route.Folder("1"), nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToRoot() = runTest {
    val nav = Navigator(Route.Home)
    navigateAndComplete(nav, Route.Space)
    navigateAndComplete(nav, Route.Folder("1"))
    nav.popToRoot()
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
        nav.performPop()
        nav.consumePopRequest()
      }
      nav.completeTransition()
      advanceUntilIdle()
      job.join()
    }
  }
}
