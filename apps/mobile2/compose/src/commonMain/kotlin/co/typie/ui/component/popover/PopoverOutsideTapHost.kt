package co.typie.ui.component.popover

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow

internal data class PopoverOutsideGestureUpdate(
  val dismiss: Boolean,
  val consumeChange: Boolean,
  val keepTracking: Boolean,
)

internal class PopoverOutsideGestureTracker(
  private val origin: Offset,
  private val touchSlop: Float,
) {
  private var isTapCandidate = true

  fun start(): PopoverOutsideGestureUpdate {
    return PopoverOutsideGestureUpdate(dismiss = true, consumeChange = false, keepTracking = true)
  }

  fun update(currentPosition: Offset, isPressed: Boolean): PopoverOutsideGestureUpdate {
    if (isTapCandidate && (currentPosition - origin).getDistance() > touchSlop) {
      isTapCandidate = false
    }

    val isTapRelease = isTapCandidate && !isPressed
    return PopoverOutsideGestureUpdate(
      dismiss = false,
      consumeChange = isTapRelease,
      keepTracking = isPressed,
    )
  }
}

@Composable
internal fun Modifier.popoverOutsideTapHost(state: PopoverOverlayState): Modifier {
  var rootWindowOffset by remember { mutableStateOf(Offset.Zero) }

  return this.onGloballyPositioned { coordinates ->
      rootWindowOffset = coordinates.positionInWindow()
    }
    .pointerInput(state, rootWindowOffset) {
      awaitEachGesture {
        // Intercept outside taps before descendants turn them into clicks, but keep drags
        // unconsumed.
        val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
        val paneBounds = state.outsideDismissPaneBoundsInWindow ?: return@awaitEachGesture
        val downPositionInWindow = down.position + rootWindowOffset
        if (paneBounds.contains(downPositionInWindow)) {
          return@awaitEachGesture
        }

        val gestureId = state.beginOutsideDismissGesture()
        try {
          val gestureTracker =
            PopoverOutsideGestureTracker(
              origin = downPositionInWindow,
              touchSlop = viewConfiguration.touchSlop,
            )
          val startUpdate = gestureTracker.start()
          if (startUpdate.dismiss) {
            state.dismissFromOutsideGesture()
          }
          if (!startUpdate.keepTracking) {
            return@awaitEachGesture
          }

          var pressed = true
          while (pressed) {
            val event = awaitPointerEvent(pass = PointerEventPass.Initial)
            val change = event.changes.find { it.id == down.id } ?: break
            val currentPositionInWindow = change.position + rootWindowOffset
            val update =
              gestureTracker.update(
                currentPosition = currentPositionInWindow,
                isPressed = change.pressed,
              )
            if (update.consumeChange) {
              change.consume()
            }
            if (!update.keepTracking) {
              break
            }
            pressed = change.pressed
          }
        } finally {
          state.endOutsideDismissGesture(gestureId)
        }
      }
    }
}
