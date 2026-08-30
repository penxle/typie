package co.typie.ui.input

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerType

@Stable
internal class PointerInputModeState {
  var nonTouchPointerActive by mutableStateOf(false)
    private set

  fun observe(event: PointerEvent): Boolean {
    nonTouchPointerActive = event.hasNonTouchPointer()
    return nonTouchPointerActive
  }

  fun leave(event: PointerEvent) {
    if (event.hasNonTouchPointer()) nonTouchPointerActive = false
  }
}

internal fun PointerEvent.hasNonTouchPointer(): Boolean = changes.any { change ->
  change.type != PointerType.Touch
}

internal fun Modifier.trackPointerInputMode(
  state: PointerInputModeState,
  onNonTouchPointerEnter: () -> Unit = {},
): Modifier =
  onPointerEvent(PointerEventType.Enter, PointerEventPass.Initial) { event ->
      if (state.observe(event)) onNonTouchPointerEnter()
    }
    .onPointerEvent(PointerEventType.Move, PointerEventPass.Initial) { event ->
      state.observe(event)
    }
    .onPointerEvent(PointerEventType.Press, PointerEventPass.Initial) { event ->
      state.observe(event)
    }
    .onPointerEvent(PointerEventType.Scroll, PointerEventPass.Initial) { event ->
      state.observe(event)
    }
    .onPointerEvent(PointerEventType.Exit, PointerEventPass.Initial) { event -> state.leave(event) }
