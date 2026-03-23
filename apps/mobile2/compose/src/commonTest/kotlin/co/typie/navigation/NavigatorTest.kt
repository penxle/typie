package co.typie.navigation

import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest

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
    nav.navigate(Route.Detail("1"))
    assertEquals(Route.Detail("1"), nav.current)
    assertEquals(Route.Home, nav.previous)
    assertTrue(nav.canPop)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun pop() = runTest {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.pop()
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
    nav.navigate(Route.Space)
    nav.navigate(Route.Detail("1"))
    nav.navigate(Route.Detail("2"))
    nav.popTo(Route.Space)
    assertEquals(Route.Space, nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToNotInStack() = runTest {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.popTo(Route.Notes)
    assertEquals(Route.Detail("1"), nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToRoot() = runTest {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Space)
    nav.navigate(Route.Detail("1"))
    nav.popToRoot()
    assertEquals(Route.Home, nav.current)
    assertEquals(1, nav.stack.size)
  }

  @Test
  fun showAndDismissModal() {
    val nav = Navigator(Route.Home)
    nav.showModal {}
    assertTrue(nav.canPop)
    assertEquals(1, nav.modals.size)
    val result = nav.dismissModal()
    assertTrue(result)
    assertEquals(0, nav.modals.size)
  }

  @Test
  fun dismissModalWhenEmpty() {
    val nav = Navigator(Route.Home)
    val result = nav.dismissModal()
    assertFalse(result)
  }

  @Test
  fun popDismissesModalFirst() = runTest {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.showModal {}
    nav.pop()
    assertEquals(0, nav.modals.size)
    assertEquals(Route.Detail("1"), nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun lastOperationOnNavigate() = runTest {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    assertEquals(NavOperation.Push, nav.lastOperation)
  }

  @Test
  fun lastOperationOnPop() = runTest {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.pop()
    assertEquals(NavOperation.Pop, nav.lastOperation)
  }
}
