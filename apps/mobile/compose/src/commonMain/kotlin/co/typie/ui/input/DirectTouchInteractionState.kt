package co.typie.ui.input

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.pointerInput

@Stable
internal class DirectTouchInteractionState(initialDirectTouchInteraction: Boolean) {
  var isDirectTouchInteraction by mutableStateOf(initialDirectTouchInteraction)
    private set

  fun recordPointerInteraction(type: PointerType) {
    isDirectTouchInteraction = type.isDirectTouchInteraction()
  }

  fun recordKeyboardInteraction() {
    isDirectTouchInteraction = false
  }
}

internal fun PointerType.isDirectTouchInteraction(): Boolean =
  this == PointerType.Touch || this == PointerType.Stylus || this == PointerType.Eraser

@OptIn(ExperimentalComposeUiApi::class)
internal fun Modifier.trackDirectTouchInteraction(state: DirectTouchInteractionState): Modifier =
  pointerInput(state) {
      awaitPointerEventScope {
        while (true) {
          val event = awaitPointerEvent(PointerEventPass.Initial)
          when (event.type) {
            PointerEventType.Press ->
              event.changes
                .lastOrNull { change -> change.pressed && !change.previousPressed }
                ?.let { change -> state.recordPointerInteraction(change.type) }
            PointerEventType.Scroll,
            PointerEventType.PanStart,
            PointerEventType.PanMove,
            PointerEventType.ScaleStart,
            PointerEventType.ScaleChange -> state.recordPointerInteraction(PointerType.Mouse)
            else -> Unit
          }
        }
      }
    }
    .onPreviewKeyEvent { event ->
      if (event.type == KeyEventType.KeyDown) {
        state.recordKeyboardInteraction()
      }
      false
    }

internal val LocalDirectTouchInteractionState =
  staticCompositionLocalOf<DirectTouchInteractionState> {
    error("No DirectTouchInteractionState provided")
  }
