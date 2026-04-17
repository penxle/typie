package co.typie.ui.component.popover

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.PointerInputChange
import kotlinx.coroutines.withTimeoutOrNull

internal data class PressGestureSession(
  val positionInWindow: Offset,
  val isArmed: Boolean,
  val isReleased: Boolean,
)

@Stable
internal class PressGestureSessionState {
  var session by mutableStateOf<PressGestureSession?>(null)
    private set

  fun publish(session: PressGestureSession) {
    this.session = session
  }

  fun clear() {
    session = null
  }
}

@Composable
internal fun rememberPressGestureSessionState(): PressGestureSessionState {
  return remember { PressGestureSessionState() }
}

internal suspend fun AwaitPointerEventScope.trackPressGestureSession(
  pointerId: PointerId,
  initialPositionInWindow: Offset,
  downUptimeMillis: Long,
  armDelayMillis: Long,
  resolvePositionInWindow:
    (change: PointerInputChange, previousPositionInWindow: Offset) -> Offset?,
  onSession: (session: PressGestureSession, change: PointerInputChange?) -> Unit,
): Boolean {
  var isArmed = false
  var isPressed = true
  var currentPositionInWindow = initialPositionInWindow
  var elapsedMillis = 0L

  fun publish(change: PointerInputChange?) {
    onSession(
      PressGestureSession(
        positionInWindow = currentPositionInWindow,
        isArmed = isArmed,
        isReleased = !isPressed,
      ),
      change,
    )
  }

  publish(change = null)

  while (isPressed) {
    val event =
      if (!isArmed && elapsedMillis < armDelayMillis) {
        withTimeoutOrNull(armDelayMillis - elapsedMillis) { awaitPointerEvent() }
      } else {
        awaitPointerEvent()
      }

    if (event == null) {
      isArmed = true
      publish(change = null)
      continue
    }

    val change = event.changes.find { it.id == pointerId } ?: return false
    currentPositionInWindow =
      resolvePositionInWindow(change, currentPositionInWindow) ?: currentPositionInWindow
    elapsedMillis = change.uptimeMillis - downUptimeMillis

    if (change.pressed && !isArmed && elapsedMillis >= armDelayMillis) {
      isArmed = true
    }

    isPressed = change.pressed
    publish(change = change)
  }

  return true
}
