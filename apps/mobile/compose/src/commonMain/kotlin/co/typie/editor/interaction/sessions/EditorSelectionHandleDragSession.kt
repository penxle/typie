package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.semantics.dispatchSelectionHandleExtension

internal class EditorSelectionHandleDragSession {
  private var pendingContext: EditorPendingSelectionHandleDrag? = null
  private var dragContext: EditorSelectionHandleDragContext? = null

  val pendingDrag: Boolean
    get() = pendingContext != null

  val activeDrag: Boolean
    get() = dragContext != null

  fun isActiveType(type: EditorSelectionHandleType): Boolean = dragContext?.type == type

  fun beginPending(type: EditorSelectionHandleType, touchPosition: Offset): Boolean {
    if (activeDrag) {
      return false
    }
    pendingContext = EditorPendingSelectionHandleDrag(type = type, touchPosition = touchPosition)
    return true
  }

  fun beginDrag(
    type: EditorSelectionHandleType,
    touchPosition: Offset,
    context: EditorGestureContext,
  ): Boolean {
    if (activeDrag) {
      return false
    }

    val endpoints = context.editor.selectionEndpoints() ?: return false
    val handle = endpoints.handle(type)
    val anchor = endpoints.anchorFor(type)
    val startTouchPosition =
      pendingContext?.takeIf { it.type == type }?.touchPosition ?: touchPosition
    val handleCenter =
      context.geometry.resolvePagePosition(
        page = handle.pageIdx,
        x = handle.rect.x,
        y = handle.rect.y + handle.rect.height / 2f,
      ) ?: touchPosition

    pendingContext = null
    dragContext =
      EditorSelectionHandleDragContext(
        type = type,
        startTouchPosition = startTouchPosition,
        startHandlePosition = handleCenter,
        anchor = anchor,
      )
    return true
  }

  fun update(
    type: EditorSelectionHandleType,
    touchPosition: Offset,
    context: EditorGestureContext,
  ): Boolean {
    val drag = dragContext ?: return false
    if (drag.type != type) {
      return false
    }

    val selectionPosition = drag.startHandlePosition + (touchPosition - drag.startTouchPosition)
    context.semantics.magnifier.show(selectionPosition)
    context.semantics.edgeAutoScroll.trackSelectionHandle(
      edgePosition = touchPosition,
      dispatchPosition = selectionPosition,
      anchor = drag.anchor,
      context = context,
    )
    val point = context.geometry.resolvePoint(positionInNode = selectionPosition) ?: return false
    if (point.page < 0) {
      return false
    }
    return context.editor.dispatchSelectionHandleExtension(point = point, anchor = drag.anchor)
  }

  fun reset() {
    pendingContext = null
    dragContext = null
  }
}

private data class EditorPendingSelectionHandleDrag(
  val type: EditorSelectionHandleType,
  val touchPosition: Offset,
)

private data class EditorSelectionHandleDragContext(
  val type: EditorSelectionHandleType,
  val startTouchPosition: Offset,
  val startHandlePosition: Offset,
  val anchor: PageRect,
)

private fun SelectionEndpoints.handle(type: EditorSelectionHandleType) =
  when (type) {
    EditorSelectionHandleType.From -> from
    EditorSelectionHandleType.To -> to
  }

private fun SelectionEndpoints.anchorFor(type: EditorSelectionHandleType) =
  when (type) {
    EditorSelectionHandleType.From -> to
    EditorSelectionHandleType.To -> from
  }
