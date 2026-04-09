package co.typie.screen.entity_move

enum class MoveDestinationNavigationDirection {
  None,
  Forward,
  Backward,
}

fun resolveMoveDestinationNavigationDirection(
  currentDestinationId: String?,
  nextDestinationId: String?,
  childDestinationIds: Set<String>,
): MoveDestinationNavigationDirection {
  if (currentDestinationId == nextDestinationId) {
    return MoveDestinationNavigationDirection.None
  }

  if (nextDestinationId != null && nextDestinationId in childDestinationIds) {
    return MoveDestinationNavigationDirection.Forward
  }

  return if (currentDestinationId != null) {
    MoveDestinationNavigationDirection.Backward
  } else {
    MoveDestinationNavigationDirection.None
  }
}
