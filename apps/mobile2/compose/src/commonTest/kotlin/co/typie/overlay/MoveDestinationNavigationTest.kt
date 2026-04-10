package co.typie.overlay

import kotlin.test.Test
import kotlin.test.assertEquals

class MoveDestinationNavigationTest {

  @Test
  fun navigatingIntoChildUsesForwardTransition() {
    val direction = resolveMoveDestinationNavigationDirection(
      currentDestinationId = "folder-1",
      nextDestinationId = "folder-2",
      childDestinationIds = setOf("folder-2", "folder-3"),
    )

    assertEquals(MoveDestinationNavigationDirection.Forward, direction)
  }

  @Test
  fun navigatingUpToParentUsesBackwardTransition() {
    val direction = resolveMoveDestinationNavigationDirection(
      currentDestinationId = "folder-2",
      nextDestinationId = "folder-1",
      childDestinationIds = setOf("folder-3"),
    )

    assertEquals(MoveDestinationNavigationDirection.Backward, direction)
  }

  @Test
  fun navigatingToRootUsesBackwardTransition() {
    val direction = resolveMoveDestinationNavigationDirection(
      currentDestinationId = "folder-1",
      nextDestinationId = null,
      childDestinationIds = setOf("folder-2"),
    )

    assertEquals(MoveDestinationNavigationDirection.Backward, direction)
  }

  @Test
  fun stayingOnSameDestinationUsesNoneTransition() {
    val direction = resolveMoveDestinationNavigationDirection(
      currentDestinationId = "folder-1",
      nextDestinationId = "folder-1",
      childDestinationIds = setOf("folder-2"),
    )

    assertEquals(MoveDestinationNavigationDirection.None, direction)
  }
}
