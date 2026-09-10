package co.typie.ui.component.popover

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockHandle
import kotlinx.coroutines.withTimeoutOrNull

/** Keeps the opening press alive while a popover or submenu becomes interactive. */
@Composable
internal fun rememberPopoverTriggerInputModifier(
  interactionSource: MutableInteractionSource,
  canOpen: () -> Boolean,
  positionInWindow: (Offset) -> Offset?,
  onOpen: () -> Unit,
  onSession: (PressGestureSession?) -> Unit,
): Modifier {
  val canOpenState = rememberUpdatedState(canOpen)
  val positionInWindowState = rememberUpdatedState(positionInWindow)
  val onOpenState = rememberUpdatedState(onOpen)
  val onSessionState = rememberUpdatedState(onSession)
  val scrollGestureLockState = LocalScrollGestureLockState.current
  return Modifier.pointerInput(interactionSource, scrollGestureLockState) {
    awaitEachGesture {
      val initialDown = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
      if (initialDown.isConsumed) return@awaitEachGesture
      // A clickable anchor may consume Main; an ancestor's Initial consumption owns dismissal.
      val press = awaitPointerEvent(PointerEventPass.Main).changes.first { it.id == initialDown.id }
      if (!canOpenState.value()) return@awaitEachGesture

      val initialPositionInWindow =
        positionInWindowState.value(press.position) ?: return@awaitEachGesture
      val pressInteraction = PressInteraction.Press(press.position)
      interactionSource.tryEmit(pressInteraction)
      val openTrigger =
        awaitPopoverOpenOrCancellation(
          press = press,
          initialPositionInWindow = initialPositionInWindow,
          touchSlop = viewConfiguration.touchSlop,
          armDelayMillis = PopoverDefaults.ArmDelayMs,
          resolvePositionInWindow = { change ->
            positionInWindowState.value(change.position) ?: initialPositionInWindow
          },
        )

      when (openTrigger) {
        null -> {
          interactionSource.tryEmit(PressInteraction.Cancel(pressInteraction))
          return@awaitEachGesture
        }
        is PopoverOpenTrigger.Tap -> {
          interactionSource.tryEmit(PressInteraction.Release(pressInteraction))
          onOpenState.value()
          openTrigger.upChange.consume()
          return@awaitEachGesture
        }
        PopoverOpenTrigger.Pressed -> {}
      }

      var scrollLockHandle: ScrollGestureLockHandle? = null
      var released = false

      try {
        scrollLockHandle = scrollGestureLockState.acquire()
        onOpenState.value()
        released =
          trackPressGestureSession(
            pointerId = press.id,
            initialPositionInWindow = initialPositionInWindow,
            downUptimeMillis = press.uptimeMillis,
            armDelayMillis = PopoverDefaults.ArmDelayMs,
            resolvePositionInWindow = { nextChange, previous ->
              positionInWindowState.value(nextChange.position) ?: previous
            },
          ) { session, change ->
            onSessionState.value(session)
            change?.consume()
          }
      } finally {
        interactionSource.tryEmit(
          if (released) {
            PressInteraction.Release(pressInteraction)
          } else {
            PressInteraction.Cancel(pressInteraction)
          }
        )
        if (!released) {
          onSessionState.value(null)
        }
        scrollLockHandle?.release()
      }
    }
  }
}

private sealed interface PopoverOpenTrigger {
  data class Tap(val upChange: PointerInputChange) : PopoverOpenTrigger

  data object Pressed : PopoverOpenTrigger
}

private suspend fun AwaitPointerEventScope.awaitPopoverOpenOrCancellation(
  press: PointerInputChange,
  initialPositionInWindow: Offset,
  touchSlop: Float,
  armDelayMillis: Long,
  resolvePositionInWindow: (PointerInputChange) -> Offset,
): PopoverOpenTrigger? {
  var elapsedMillis = 0L

  while (elapsedMillis < armDelayMillis) {
    val event = withTimeoutOrNull(armDelayMillis - elapsedMillis) { awaitPointerEvent() }
    if (event == null) {
      return PopoverOpenTrigger.Pressed
    }

    val change = event.changes.find { it.id == press.id } ?: return null
    val currentPositionInWindow = resolvePositionInWindow(change)
    elapsedMillis = change.uptimeMillis - press.uptimeMillis
    val dragDistance = (currentPositionInWindow - initialPositionInWindow).getDistance()

    if (dragDistance > touchSlop) {
      return null
    }
    if (!change.pressed) {
      return PopoverOpenTrigger.Tap(change)
    }
  }

  return PopoverOpenTrigger.Pressed
}
