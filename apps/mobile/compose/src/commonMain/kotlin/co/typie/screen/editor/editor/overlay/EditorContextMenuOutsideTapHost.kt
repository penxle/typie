package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.platform.LocalViewConfiguration
import co.typie.editor.interaction.EditorInteractionGeometry
import co.typie.editor.runtime.EditorContextMenuState
import co.typie.ui.input.WindowInputHandler

@Composable
internal fun EditorContextMenuOutsideTapHost(
  state: EditorContextMenuState,
  geometry: EditorInteractionGeometry,
  enabled: Boolean = true,
) {
  val touchSlop = LocalViewConfiguration.current.touchSlop
  val handler =
    remember(state, geometry, touchSlop) { EditorContextMenuOutsideTap(state, geometry, touchSlop) }
  WindowInputHandler(enabled, onPointerEvent = handler::onPointerEvent, onCancel = handler::cancel)
}

private class EditorContextMenuOutsideTap(
  private val state: EditorContextMenuState,
  private val geometry: EditorInteractionGeometry,
  private val touchSlop: Float,
) {
  private var down: PointerInputChange? = null
  private var moved = false

  fun onPointerEvent(event: PointerEvent, layout: LayoutCoordinates) {
    val previousDown = down
    if (previousDown != null) {
      val change = event.changes.firstOrNull { it.id == previousDown.id } ?: return
      moved = moved || (change.position - previousDown.position).getDistance() > touchSlop
      if (!change.pressed) {
        if (!moved) change.consume()
        cancel()
      }
      return
    }
    if (event.type != PointerEventType.Press || !state.visible) return
    val change =
      event.changes.firstOrNull { it.pressed && !it.previousPressed && !it.isConsumed } ?: return
    val bounds = state.boundsInWindow ?: return
    if (bounds.contains(layout.localToWindow(change.position))) return
    if (state.pointerPosition == null) {
      // The editor's touch-selection semantics already dismiss its menu on pointer down.
      if (!geometry.containsDocumentInteraction(layout.localToRoot(change.position))) state.hide()
      return
    }
    down = change
    moved = false
    state.beginOutsideDismissGesture(change.id.value)
    // Controls cannot click through. EditorInteractions explicitly admits this pointer for
    // handle dragging and viewport panning, with tap/long-press selection disabled.
    change.consume()
  }

  fun cancel() {
    down?.let { state.endOutsideDismissGesture(it.id.value) }
    down = null
    moved = false
  }
}
