package co.typie.ui.input

import androidx.compose.foundation.focusable
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned

internal val LocalWindowInputState = staticCompositionLocalOf<WindowInputState?> { null }

internal class WindowInputState {
  val handlers = mutableStateListOf<WindowInputHandler>()
}

internal class WindowInputHandler(
  val onKeyEvent: ((KeyEvent) -> Boolean)?,
  val onPointerEvent: ((PointerEvent, LayoutCoordinates) -> Unit)?,
  val onCancel: () -> Unit,
)

/** Active screens register at the window, including their portaled top bars and foregrounds. */
@Composable
internal fun WindowInputHandler(
  enabled: Boolean,
  onKeyEvent: ((KeyEvent) -> Boolean)? = null,
  onPointerEvent: ((PointerEvent, LayoutCoordinates) -> Unit)? = null,
  onCancel: () -> Unit = {},
) {
  val state = LocalWindowInputState.current ?: return
  val currentKey by rememberUpdatedState(onKeyEvent)
  val currentPointer by rememberUpdatedState(onPointerEvent)
  val cancel by rememberUpdatedState(onCancel)
  val handler =
    remember(onKeyEvent != null, onPointerEvent != null) {
      WindowInputHandler(
        onKeyEvent =
          if (onKeyEvent != null) { event -> currentKey?.invoke(event) == true } else null,
        onPointerEvent =
          if (onPointerEvent != null)
            { event, coordinates ->
              currentPointer?.invoke(event, coordinates)
              Unit
            }
          else null,
        onCancel = { cancel() },
      )
    }
  DisposableEffect(state, enabled, handler) {
    if (enabled) state.handlers.add(handler)
    onDispose {
      state.handlers.remove(handler)
      cancel()
    }
  }
}

@Composable
internal fun Modifier.windowInput(state: WindowInputState): Modifier {
  val focusRequester = remember { FocusRequester() }
  var hasFocus by remember { mutableStateOf(false) }
  var coordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }
  val handlesKeys = state.handlers.any { it.onKeyEvent != null }
  LaunchedEffect(handlesKeys, hasFocus) {
    if (handlesKeys && !hasFocus) {
      withFrameNanos {}
      if (!hasFocus) focusRequester.requestFocus()
    }
  }
  return onGloballyPositioned { coordinates = it }
    .pointerInput(state) {
      try {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent(PointerEventPass.Initial)
            val layout = coordinates?.takeIf { it.isAttached } ?: continue
            // Later registrations run first. Pointer handlers share consumption and all observe
            // the event; key dispatch below stops at the first handler that accepts it.
            state.handlers.toList().asReversed().forEach {
              it.onPointerEvent?.invoke(event, layout)
            }
          }
        }
      } finally {
        state.handlers.toList().forEach { it.onCancel() }
      }
    }
    .onPreviewKeyEvent { event ->
      state.handlers.toList().asReversed().any { it.onKeyEvent?.invoke(event) == true }
    }
    .focusRequester(focusRequester)
    .onFocusChanged { hasFocus = it.hasFocus }
    .focusable(enabled = handlesKeys)
}
