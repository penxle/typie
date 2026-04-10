package co.typie.route

import kotlin.test.Test
import kotlin.test.assertEquals

class RouteTransitionStyleTest {

  @Test
  fun `home search fades in from home`() {
    assertEquals(RouteTransitionStyle.Fade, Route.Home.transitionStyleTo(Route.HomeSearch))
  }

  @Test
  fun `non search routes keep slide transition`() {
    assertEquals(
      RouteTransitionStyle.Slide,
      Route.HomeSearch.transitionStyleTo(Route.Editor("doc-1")),
    )
  }
}
