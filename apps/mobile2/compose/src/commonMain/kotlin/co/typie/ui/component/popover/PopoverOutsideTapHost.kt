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

internal val LocalPopoverOutsideTapHostState = staticCompositionLocalOf<PopoverOutsideTapHostState?> { null }

private data class PopoverOutsideTapRegistration(
  val token: Any,
  val paneBounds: Rect?,
  val onDismiss: () -> Unit,
)

@Stable
class PopoverOutsideTapHostState {
  private var registration by mutableStateOf<PopoverOutsideTapRegistration?>(null)

  fun register(): PopoverOutsideTapHostHandle {
    return PopoverOutsideTapHostHandle(
      state = this,
      token = Any(),
    )
  }

  internal fun currentRegistration(): Pair<Rect, () -> Unit>? {
    val active = registration ?: return null
    val paneBounds = active.paneBounds ?: return null
    return paneBounds to active.onDismiss
  }

  internal fun update(
    token: Any,
    paneBounds: Rect?,
    onDismiss: () -> Unit,
  ) {
    registration = PopoverOutsideTapRegistration(
      token = token,
      paneBounds = paneBounds,
      onDismiss = onDismiss,
    )
  }

  internal fun clear(token: Any) {
    if (registration?.token == token) {
      registration = null
    }
  }
}

@Stable
class PopoverOutsideTapHostHandle internal constructor(
  private val state: PopoverOutsideTapHostState,
  private val token: Any,
) {
  fun update(
    paneBounds: Rect?,
    onDismiss: () -> Unit,
  ) {
    state.update(
      token = token,
      paneBounds = paneBounds,
      onDismiss = onDismiss,
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
    modifier = Modifier
      .fillMaxSize()
      .onGloballyPositioned { coordinates ->
        rootWindowOffset = coordinates.positionInWindow()
      }
      .pointerInput(state, rootWindowOffset) {
        awaitEachGesture {
          val down = awaitFirstDown(
            requireUnconsumed = false,
            pass = PointerEventPass.Initial,
          )
          val (paneBounds, onDismiss) = state.currentRegistration() ?: return@awaitEachGesture
          val downPositionInWindow = down.position + rootWindowOffset
          if (paneBounds.contains(downPositionInWindow)) {
            return@awaitEachGesture
          }

          down.consume()
          onDismiss()

          var pressed = true
          while (pressed) {
            val event = awaitPointerEvent(pass = PointerEventPass.Initial)
            val change = event.changes.find { it.id == down.id } ?: break
            pressed = change.pressed
          }
        }
      },
  ) {
    CompositionLocalProvider(LocalPopoverOutsideTapHostState provides state) {
      content()
    }
  }
}
