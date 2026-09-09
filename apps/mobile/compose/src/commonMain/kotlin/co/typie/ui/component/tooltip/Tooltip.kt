package co.typie.ui.component.tooltip

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import co.typie.platform.LocalHardwareKeyboardConnected

internal val LocalTooltipState = staticCompositionLocalOf<TooltipState?> { null }

/** Describes a control without changing its click, long-press, or focus acquisition behavior. */
@Composable
fun Modifier.tooltip(
  text: String,
  shortcut: String? = null,
  enabled: Boolean = true,
  placement: TooltipPlacement = TooltipPlacement.Below,
): Modifier {
  val state = LocalTooltipState.current ?: return this
  val anchor = remember { TooltipAnchor() }
  val keyboardConnected = LocalHardwareKeyboardConnected.current
  SideEffect {
    anchor.text = text
    anchor.placement = placement
    anchor.shortcut = shortcut.takeIf { keyboardConnected }
  }
  val available = enabled && text.isNotBlank()
  DisposableEffect(state, anchor, available) { onDispose { state.remove(anchor) } }
  if (!available) return this

  return onGloballyPositioned {
      anchor.coordinates = it
      anchor.bounds = it.boundsInWindow(clipBounds = false)
    }
    .pointerInput(state, anchor) {
      awaitPointerEventScope {
        var press: PointerInputChange? = null
        var dragged = false
        while (true) {
          val event = awaitPointerEvent(PointerEventPass.Initial)
          val initial = press
          val change = event.changes.firstOrNull { it.id == initial?.id }
          if (initial != null && change != null) {
            if ((change.position - initial.position).getDistance() > viewConfiguration.touchSlop)
              dragged = true
            if (!change.pressed) {
              if (
                !dragged &&
                  change.position.x in 0f..size.width.toFloat() &&
                  change.position.y in 0f..size.height.toFloat()
              )
                state.close(anchor)
              press = null
            }
          }
          when (event.type) {
            PointerEventType.Enter ->
              if (event.changes.any { it.type != PointerType.Touch && !it.pressed }) {
                state.enter(anchor)
              }
            PointerEventType.Exit -> state.leave(anchor)
            PointerEventType.Press ->
              if (press == null) {
                press = event.changes.firstOrNull { it.pressed && !it.previousPressed }
                dragged = false
              }
          }
        }
      }
    }
}
