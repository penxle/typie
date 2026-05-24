package co.typie.editor.interaction.gestures

import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerInputScope
import co.typie.editor.ext.isCollapsed
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.sessions.EditorSelectionHandleDragSession

internal enum class EditorSelectionHandleType {
  From,
  To,
}

internal class EditorSelectionHandleGesture(
  private val contextProvider: () -> EditorGestureContext,
  private val session: EditorSelectionHandleDragSession = EditorSelectionHandleDragSession(),
) {
  val pendingDrag: Boolean
    get() = session.pendingDrag

  val activeDrag: Boolean
    get() = session.activeDrag

  suspend fun PointerInputScope.detectDrag(
    type: EditorSelectionHandleType,
    positionInEditor: (Offset) -> Offset,
  ) {
    awaitEachGesture {
      var completed = false
      try {
        val down = awaitFirstDown(requireUnconsumed = false)
        val downPosition = positionInEditor(down.position)
        val touchSlop = viewConfiguration.touchSlop
        if (!handleDragDown(type = type, position = downPosition)) {
          completed = true
          return@awaitEachGesture
        }
        down.consume()

        var dragging = false

        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.firstOrNull { it.id == down.id } ?: continue
          val position = positionInEditor(change.position)

          if (!change.pressed) {
            if (dragging) {
              if (handleDragEnd(type = type)) {
                change.consume()
              }
            } else {
              cancel()
            }
            completed = true
            return@awaitEachGesture
          }

          if (!dragging) {
            change.consume()
            if ((position - downPosition).getDistance() <= touchSlop) {
              continue
            }
            if (!handleDragStart(type = type, position = position)) {
              completed = true
              return@awaitEachGesture
            }
            dragging = true
          }
          if (dragging && handleDragUpdate(type = type, position = position)) {
            change.consume()
          }
        }
      } finally {
        if (!completed) {
          cancel()
        }
      }
    }
  }

  fun handleDragDown(type: EditorSelectionHandleType, position: Offset): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      return false
    }
    if (!session.beginPending(type = type, touchPosition = position)) {
      return false
    }
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    context.effects.setScrollGestureLocked(true)
    context.uiState.contextMenu.hide()
    return true
  }

  fun handleDragStart(type: EditorSelectionHandleType, position: Offset): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      resetPointerOwnedState(context = context)
      return false
    }
    if (!session.beginDrag(type = type, touchPosition = position, context = context)) {
      resetPointerOwnedState(context = context)
      return false
    }

    context.uiState.contextMenu.hide()
    context.semantics.magnifier.hide()
    context.reduceMode(EditorInteractionEvent.SelectionHandleDragStart)
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      resetPointerOwnedState(context = context)
      return false
    }
    return true
  }

  fun handleDragUpdate(type: EditorSelectionHandleType, position: Offset): Boolean {
    val context = contextProvider()
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      return false
    }
    if (!session.isActiveType(type)) {
      return false
    }
    session.update(type = type, touchPosition = position, context = context)
    return true
  }

  fun handleDragEnd(type: EditorSelectionHandleType): Boolean {
    val context = contextProvider()
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
    if (wasDragging && !context.editor.selection.isCollapsed()) {
      context.uiState.contextMenu.show(context.editor.state)
      context.uiState.contextMenu.requestShowAfterSelectionCommit()
    }
    return wasActive
  }

  fun cancel() {
    val context = contextProvider()
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
