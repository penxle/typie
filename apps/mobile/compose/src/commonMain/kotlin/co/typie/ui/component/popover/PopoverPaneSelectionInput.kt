package co.typie.ui.component.popover

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.ext.EdgeAutoScrollController

@Composable
internal fun rememberPopoverPaneSelectionInputModifier(
  enabled: Boolean,
  positionInWindow: (Offset) -> Offset?,
  selectionState: PopoverPaneSelectionState,
  edgeAutoScrollController: EdgeAutoScrollController?,
  armDelayMillis: Long = PopoverDefaults.ArmDelayMs,
): Modifier {
  val positionInWindowState = rememberUpdatedState(positionInWindow)

  return Modifier.pointerInput(enabled, selectionState, edgeAutoScrollController, armDelayMillis) {
    awaitEachGesture {
      if (!enabled) {
        return@awaitEachGesture
      }

      val down = awaitFirstDown(requireUnconsumed = false)
      val initialPositionInWindow =
        positionInWindowState.value(down.position) ?: return@awaitEachGesture
      if (!selectionState.canHandleLocalGesture(initialPositionInWindow)) {
        return@awaitEachGesture
      }

      val touchSlop = viewConfiguration.touchSlop
      var panScrollDetected = false

      selectionState.clear()

      try {
        trackPressGestureSession(
          pointerId = down.id,
          initialPositionInWindow = initialPositionInWindow,
          downUptimeMillis = down.uptimeMillis,
          armDelayMillis = armDelayMillis,
          resolvePositionInWindow = { change, previousPositionInWindow ->
            positionInWindowState.value(change.position) ?: previousPositionInWindow
          },
        ) { session, change ->
          val currentPositionInWindow = session.positionInWindow

          if (!panScrollDetected && change != null && !session.isArmed) {
            val dragDistance = (currentPositionInWindow - initialPositionInWindow).getDistance()
            if (change.pressed && dragDistance > touchSlop) {
              panScrollDetected = true
              selectionState.clear()
              edgeAutoScrollController?.pointer = null
            }
          }

          if (!panScrollDetected && session.isArmed) {
            change?.consume()
            // Two sinks: selectionState drives the highlight; edgeAutoScrollController drives the
            // edge scroll loop.
            // Re-hit-test during scroll is reactive via item onGloballyPositioned; no scrollEpoch
            // observer needed.
            selectionState.updatePointer(currentPositionInWindow)
            edgeAutoScrollController?.pointer = currentPositionInWindow
          }

          if (change != null && !change.pressed) {
            edgeAutoScrollController?.pointer = null
            if (!panScrollDetected && session.isArmed) {
              change.consume()
              selectionState.release(currentPositionInWindow)
            }
          }
        }
      } finally {
        edgeAutoScrollController?.pointer = null
        selectionState.clear()
      }
    }
  }
}
