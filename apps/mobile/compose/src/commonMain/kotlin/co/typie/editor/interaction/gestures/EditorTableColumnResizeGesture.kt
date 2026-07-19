package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.semantics.EditorTableColumnResizePlacement
import co.typie.editor.interaction.semantics.resolveTableColumnResizePlacement

internal class EditorTableColumnResizeGesture {
  private var pointerId: Long? = null
  private var downPosition = Offset.Zero
  private var previousPosition = Offset.Zero
  private var currentPosition = Offset.Zero
  private var dragSlop = 0f
  private var dragging = false

  val pending: Boolean
    get() = pointerId != null && !dragging

  val active: Boolean
    get() = dragging

  fun updateDragSlop(dragSlop: Float) {
    this.dragSlop = dragSlop.coerceAtLeast(0f)
  }

  fun hitTest(position: Offset, context: EditorGestureContext): EditorTableColumnResizePlacement? =
    if (context.readOnly) {
      null
    } else {
      resolveTableColumnResizePlacement(editor = context.editor, geometry = context.geometry)
        ?.takeIf { candidate -> candidate.handleRects.any { rect -> rect.contains(position) } }
    }

  fun handlePointerDown(
    pointerId: Long,
    position: Offset,
    placement: EditorTableColumnResizePlacement,
    context: EditorGestureContext,
  ): Boolean {
    if (
      this.pointerId != null || !context.mode.canApply(EditorInteractionEvent.AuxiliaryGestureStart)
    ) {
      return false
    }
    this.pointerId = pointerId
    downPosition = position
    previousPosition = position
    currentPosition = position
    dragging = false
    context.semantics.tableColumnResize.press(editor = context.editor, placement = placement)
    context.uiState.contextMenu.hide()
    return true
  }

  fun handlePointerMove(pointerId: Long, position: Offset, context: EditorGestureContext): Boolean {
    if (this.pointerId != pointerId) {
      return false
    }
    previousPosition = currentPosition
    currentPosition = position
    var deltaPx = currentPosition.x - previousPosition.x
    if (!dragging) {
      if ((currentPosition - downPosition).getDistance() <= dragSlop) {
        return true
      }
      if (
        !context.mode.canApply(EditorInteractionEvent.AuxiliaryGestureStart) ||
          !context.semantics.tableColumnResize.start()
      ) {
        cancel(context = context)
        return false
      }
      dragging = true
      context.applyModeEvent(EditorInteractionEvent.AuxiliaryGestureStart)
      if (context.mode != EditorInteractionMode.AuxiliaryGesture) {
        cancel(context = context)
        return false
      }
      deltaPx = currentPosition.x - downPosition.x
    }
    context.semantics.tableColumnResize.update(deltaPx = deltaPx)
    return true
  }

  fun handlePointerUp(pointerId: Long, context: EditorGestureContext): Boolean {
    if (this.pointerId != pointerId) {
      return false
    }
    val completedDrag = dragging
    if (completedDrag) {
      context.semantics.tableColumnResize.end()
      context.applyModeEvent(EditorInteractionEvent.AuxiliaryGestureEnd)
    } else {
      context.semantics.tableColumnResize.cancel()
    }
    clear()
    return completedDrag
  }

  fun cancel(context: EditorGestureContext) {
    context.semantics.tableColumnResize.cancel()
    clear()
  }

  fun reset() {
    pointerId = null
    downPosition = Offset.Zero
    previousPosition = Offset.Zero
    currentPosition = Offset.Zero
    dragging = false
  }

  private fun clear() {
    reset()
  }
}
