package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.semantics.hasRangeSelection
import co.typie.editor.interaction.sessions.EditorSelectionHandleDragSession

internal enum class EditorSelectionHandleType {
  From,
  To,
}

internal class EditorSelectionHandleGesture(
  private val session: EditorSelectionHandleDragSession = EditorSelectionHandleDragSession()
) {
  val pendingDrag: Boolean
    get() = session.pendingDrag

  val activeDrag: Boolean
    get() = session.activeDrag

  fun handleDragDown(
    type: EditorSelectionHandleType,
    position: Offset,
    context: EditorGestureContext,
  ): Boolean {
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      return false
    }
    if (!session.beginPending(type = type, touchPosition = position)) {
      return false
    }
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    context.effects.setScrollGestureLocked(true)
    context.semantics.contextMenu.hide()
    return true
  }

  fun handleDragStart(
    type: EditorSelectionHandleType,
    position: Offset,
    context: EditorGestureContext,
  ): Boolean {
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      resetPointerOwnedState(context = context)
      return false
    }
    if (!session.beginDrag(type = type, touchPosition = position, context = context)) {
      resetPointerOwnedState(context = context)
      return false
    }

    context.semantics.contextMenu.hide()
    context.semantics.magnifier.hide()
    context.reduceMode(EditorInteractionEvent.SelectionHandleDragStart)
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      resetPointerOwnedState(context = context)
      return false
    }
    return true
  }

  fun handleDragUpdate(
    type: EditorSelectionHandleType,
    position: Offset,
    context: EditorGestureContext,
  ): Boolean {
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      return false
    }
    if (!session.isActiveType(type)) {
      return false
    }
    session.update(type = type, touchPosition = position, context = context)
    return true
  }

  fun handleDragEnd(type: EditorSelectionHandleType, context: EditorGestureContext): Boolean {
    if (activeDrag && !session.isActiveType(type)) {
      return false
    }
    val wasActive = activeDrag || pendingDrag
    val wasDragging = activeDrag
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
    if (context.mode.canApply(EditorInteractionEvent.SelectionHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.SelectionHandleDragEnd)
    }
    if (wasDragging && context.semantics.cursorMove.hasRangeSelection(context.editor)) {
      context.semantics.contextMenu.show(context.editor.state)
      context.semantics.contextMenu.requestShowAfterSelectionCommit()
    }
    return wasActive
  }

  fun cancel(context: EditorGestureContext) {
    resetPointerOwnedState(context = context)
    if (context.mode.canApply(EditorInteractionEvent.SelectionHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.SelectionHandleDragEnd)
    }
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
  }

  fun reset() = session.reset()
}
