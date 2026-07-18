package co.typie.navigation

import androidx.compose.ui.geometry.Offset
import kotlin.math.abs
import kotlin.math.max

internal sealed interface NavigationPopActivation {
  data object Pending : NavigationPopActivation

  data object Rejected : NavigationPopActivation

  data class Ready(val overshootX: Float) : NavigationPopActivation
}

internal fun resolveNavigationPopActivation(
  dragFromStart: Offset,
  activationDistance: Float,
): NavigationPopActivation {
  val distance = activationDistance.coerceAtLeast(0f)
  if (max(abs(dragFromStart.x), abs(dragFromStart.y)) < distance) {
    return NavigationPopActivation.Pending
  }
  if (dragFromStart.x <= 0f || abs(dragFromStart.x) <= abs(dragFromStart.y)) {
    return NavigationPopActivation.Rejected
  }
  return NavigationPopActivation.Ready(overshootX = (dragFromStart.x - distance).coerceAtLeast(0f))
}

internal class NavigationPopGestureSession {
  private var state = State.Possible
  private var multiTouchRejected = false
  private var systemBackZoneRejected = false
  private var pressedDragPointerCount = 0

  val isClaimed: Boolean
    get() = state == State.Claimed

  val isSystemBackZoneRejected: Boolean
    get() = systemBackZoneRejected

  val hasPressedDragPointer: Boolean
    get() = pressedDragPointerCount > 0

  val isCurrentSequenceRejected: Boolean
    get() = multiTouchRejected || systemBackZoneRejected || state == State.Rejected

  fun tryClaim(initialDrag: Offset, childConsumed: Boolean): Boolean {
    if (isCurrentSequenceRejected || state != State.Possible) {
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

  fun rejectCurrentSequence() {
    state = State.Rejected
  }

  fun updatePressedDragPointerCount(count: Int, downInSystemBackZone: Boolean = false): Boolean {
    val wasMultiTouchRejected = multiTouchRejected
    val previousCount = pressedDragPointerCount
    pressedDragPointerCount = count
    when {
      count > 1 -> {
        multiTouchRejected = true
        state = State.Rejected
      }
      count == 1 && previousCount == 0 -> {
        multiTouchRejected = false
        systemBackZoneRejected = downInSystemBackZone
        if (state != State.Claimed) {
          state = if (systemBackZoneRejected) State.Rejected else State.Possible
        }
      }
      count == 0 && multiTouchRejected -> state = State.Rejected
      count == 0 && state != State.Claimed && state != State.Rejected -> state = State.Possible
    }
    return !wasMultiTouchRejected && multiTouchRejected
  }

  fun reset() {
    state = if (isCurrentSequenceRejected) State.Rejected else State.Possible
  }

  private enum class State {
    Possible,
    Claimed,
    Rejected,
  }
}

private fun Offset.isDominantRightDrag(): Boolean = x > 0f && abs(x) > abs(y)
