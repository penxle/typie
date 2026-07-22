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
  private var currentPosition = Offset.Zero
  private var dragSlop = 0f
  private var dragging = false

  val pending: Boolean
    get() = pointerId != null && !dragging

  val active: Boolean
    get() = dragging

  fun shouldStartDrag(pointerId: Long, position: Offset): Boolean =
    this.pointerId == pointerId && pending && (position - downPosition).getDistance() > dragSlop

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
    currentPosition = position
    dragging = false
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    context.semantics.tableColumnResize.press(editor = context.editor, placement = placement)
    context.semantics.contextMenu.hide()
    return true
  }

  fun handlePointerMove(pointerId: Long, position: Offset, context: EditorGestureContext): Boolean {
    if (this.pointerId != pointerId) {
      return false
    }
    if (!dragging) {
      if (!shouldStartDrag(pointerId = pointerId, position = position)) {
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
      context.effects.setScrollGestureLocked(true)
    }
    moveTo(position = position, context = context)
    context.semantics.edgeAutoScroll.track(
      edgePosition = currentPosition,
      context = context,
      onScroll = { scrolledPosition -> moveTo(position = scrolledPosition, context = context) },
    )
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
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    clear()
    return completedDrag
  }

  fun cancel(context: EditorGestureContext) {
    context.semantics.tableColumnResize.cancel()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    clear()
  }

  fun reset() {
    pointerId = null
    downPosition = Offset.Zero
    currentPosition = Offset.Zero
    dragging = false
  }

  private fun moveTo(position: Offset, context: EditorGestureContext) {
    val deltaPx = position.x - currentPosition.x
    currentPosition = position
    if (deltaPx != 0f) {
      context.semantics.tableColumnResize.update(deltaPx = deltaPx)
    }
  }

  private fun clear() {
    reset()
  }
}
