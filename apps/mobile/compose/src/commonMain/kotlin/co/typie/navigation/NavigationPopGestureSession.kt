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
  private var pressedDragPointerCount = 0

  val isClaimed: Boolean
    get() = state == State.Claimed

  val hasPressedDragPointer: Boolean
    get() = pressedDragPointerCount > 0

  val isCurrentSequenceRejected: Boolean
    get() = state == State.Rejected

  fun tryClaim(): Boolean {
    if (state != State.Possible) return false
    state = State.Claimed
    return true
  }

  fun rejectCurrentSequence() {
    state = State.Rejected
  }

  fun updatePressedDragPointerCount(count: Int, downInSystemBackZone: Boolean = false) {
    val previousCount = pressedDragPointerCount
    pressedDragPointerCount = count
    when {
      count > 1 -> state = State.Rejected
      count == 1 && previousCount == 0 -> {
        if (state != State.Claimed) {
          state = if (downInSystemBackZone) State.Rejected else State.Possible
        }
      }
    }
  }

  fun reset() {
    if (state == State.Claimed) state = State.Possible
  }

  private enum class State {
    Possible,
    Claimed,
    Rejected,
  }
}
