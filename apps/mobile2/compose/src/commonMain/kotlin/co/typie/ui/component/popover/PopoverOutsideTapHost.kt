package co.typie.ui.component.popover

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow

internal val LocalPopoverOutsideTapHostState =
  staticCompositionLocalOf<PopoverOutsideTapHostState?> { null }

internal data class PopoverOutsideTapRegistration(
  val token: Any,
  val paneBounds: Rect?,
  val onDismiss: () -> Unit,
  val onDismissGestureFinished: () -> Unit,
)

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

@Stable
class PopoverOutsideTapHostState {
  private var registration by mutableStateOf<PopoverOutsideTapRegistration?>(null)

  fun register(): PopoverOutsideTapHostHandle {
    return PopoverOutsideTapHostHandle(state = this, token = Any())
  }

  internal fun currentRegistration(): PopoverOutsideTapRegistration? {
    val active = registration ?: return null
    return if (active.paneBounds == null) null else active
  }

  internal fun update(
    token: Any,
    paneBounds: Rect?,
    onDismiss: () -> Unit,
    onDismissGestureFinished: () -> Unit,
  ) {
    registration =
      PopoverOutsideTapRegistration(
        token = token,
        paneBounds = paneBounds,
        onDismiss = onDismiss,
        onDismissGestureFinished = onDismissGestureFinished,
      )
  }

  internal fun clear(token: Any) {
    if (registration?.token == token) {
      registration = null
    }
  }
}

@Stable
class PopoverOutsideTapHostHandle
internal constructor(private val state: PopoverOutsideTapHostState, private val token: Any) {
  fun update(paneBounds: Rect?, onDismiss: () -> Unit, onDismissGestureFinished: () -> Unit) {
    state.update(
      token = token,
      paneBounds = paneBounds,
      onDismiss = onDismiss,
      onDismissGestureFinished = onDismissGestureFinished,
    )
  }

  fun clear() {
    state.clear(token)
  }
}

@Composable
internal fun PopoverOutsideTapHost(content: @Composable () -> Unit) {
  val state = remember { PopoverOutsideTapHostState() }
  var rootWindowOffset by remember { mutableStateOf(Offset.Zero) }

  Box(
    modifier =
      Modifier.fillMaxSize()
        .onGloballyPositioned { coordinates -> rootWindowOffset = coordinates.positionInWindow() }
        .pointerInput(state, rootWindowOffset) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Final)
            val registration = state.currentRegistration() ?: return@awaitEachGesture
            val paneBounds = registration.paneBounds ?: return@awaitEachGesture
            val downPositionInWindow = down.position + rootWindowOffset
            if (paneBounds.contains(downPositionInWindow)) {
              return@awaitEachGesture
            }

            val gestureTracker =
              PopoverOutsideGestureTracker(
                origin = downPositionInWindow,
                touchSlop = viewConfiguration.touchSlop,
              )
            val startUpdate = gestureTracker.start()
            if (startUpdate.dismiss) {
              registration.onDismiss()
            }
            if (!startUpdate.keepTracking) {
              registration.onDismissGestureFinished()
              return@awaitEachGesture
            }

            var pressed = true
            while (pressed) {
              val event = awaitPointerEvent(pass = PointerEventPass.Final)
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
            registration.onDismissGestureFinished()
          }
        }
  ) {
    CompositionLocalProvider(LocalPopoverOutsideTapHostState provides state) { content() }
  }
}
