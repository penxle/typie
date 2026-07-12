package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import kotlin.math.abs

internal class NavigationPopGestureSession {
  private var state = State.Possible

  val isClaimed: Boolean
    get() = state == State.Claimed

  fun tryClaim(initialDrag: Offset, childConsumed: Boolean): Boolean {
    if (state != State.Possible) {
      return false
    }
    if (initialDrag == Offset.Zero && !childConsumed) {
      return false
    }
    state =
      if (!childConsumed && initialDrag.isDominantRightDrag()) {
        State.Claimed
      } else {
        State.Rejected
      }
    return isClaimed
  }

  fun reset() {
    state = State.Possible
  }

  private enum class State {
    Possible,
    Claimed,
    Rejected,
  }
}

private fun Offset.isDominantRightDrag(): Boolean = x > 0f && abs(x) > abs(y)
