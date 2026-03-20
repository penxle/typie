package co.typie.navigation

import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

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
  fun navigate() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    assertEquals(Route.Detail("1"), nav.current)
    assertEquals(Route.Home, nav.previous)
    assertTrue(nav.canPop)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun pop() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    val result = nav.pop()
    assertTrue(result)
    assertEquals(Route.Home, nav.current)
    assertFalse(nav.canPop)
  }

  @Test
  fun popAtRoot() {
    val nav = Navigator(Route.Home)
    val result = nav.pop()
    assertFalse(result)
    assertEquals(Route.Home, nav.current)
  }

  @Test
  fun popTo() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Space)
    nav.navigate(Route.Detail("1"))
    nav.navigate(Route.Detail("2"))
    nav.popTo(Route.Space)
    assertEquals(Route.Space, nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToNotInStack() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.popTo(Route.Notes)
    assertEquals(Route.Detail("1"), nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun popToRoot() {
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
  fun popDismissesModalFirst() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.showModal {}
    nav.pop()
    assertEquals(0, nav.modals.size)
    assertEquals(Route.Detail("1"), nav.current)
    assertEquals(2, nav.stack.size)
  }

  @Test
  fun lastOperationOnNavigate() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    assertEquals(NavOperation.Push, nav.lastOperation)
  }

  @Test
  fun lastOperationOnPop() {
    val nav = Navigator(Route.Home)
    nav.navigate(Route.Detail("1"))
    nav.pop()
    assertEquals(NavOperation.Pop, nav.lastOperation)
  }
}
