package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import kotlin.math.abs

internal class NavigationPopGestureSession {
  private var state = State.Possible
  private var multiTouchRejected = false
  private var pressedPointerCount = 0

  val isClaimed: Boolean
    get() = state == State.Claimed

  val isMultiTouchRejected: Boolean
    get() = multiTouchRejected

  fun tryClaim(initialDrag: Offset, childConsumed: Boolean): Boolean {
    if (multiTouchRejected || state != State.Possible) {
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

  fun updatePressedPointerCount(count: Int): Boolean {
    val wasMultiTouchRejected = multiTouchRejected
    val previousCount = pressedPointerCount
    pressedPointerCount = count
    when {
      count > 1 -> {
        multiTouchRejected = true
        state = State.Rejected
      }
      count == 1 && previousCount == 0 -> {
        multiTouchRejected = false
        if (state != State.Claimed) {
          state = State.Possible
        }
      }
      count == 0 && multiTouchRejected -> state = State.Rejected
      count == 0 && state != State.Claimed -> state = State.Possible
    }
    return !wasMultiTouchRejected && multiTouchRejected
  }

  fun reset() {
    state = if (multiTouchRejected) State.Rejected else State.Possible
  }

  private enum class State {
    Possible,
    Claimed,
    Rejected,
  }
}

private fun Offset.isDominantRightDrag(): Boolean = x > 0f && abs(x) > abs(y)
