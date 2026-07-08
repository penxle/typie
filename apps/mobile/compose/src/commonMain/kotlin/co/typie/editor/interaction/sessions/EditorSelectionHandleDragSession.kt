package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.semantics.dispatchSelectionHandleExtension

internal class EditorSelectionHandleDragSession {
  private var pendingContext: EditorPendingSelectionHandleDrag? = null
  private var dragContext: EditorSelectionHandleDragContext? = null

  val pendingDrag: Boolean
    get() = pendingContext != null

  val pendingType: EditorSelectionHandleType?
    get() = pendingContext?.type

  val activeDrag: Boolean
    get() = dragContext != null

  val activeType: EditorSelectionHandleType?
    get() = dragContext?.type

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
    val handle =
      when (type) {
        EditorSelectionHandleType.From -> endpoints.from
        EditorSelectionHandleType.To -> endpoints.to
      }
    val anchor =
      when (type) {
        EditorSelectionHandleType.From -> endpoints.toPosition
        EditorSelectionHandleType.To -> endpoints.fromPosition
      }
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
        baseSelection = null,
      )
    return true
  }

  fun adoptDrag(
    type: EditorSelectionHandleType,
    touchPosition: Offset,
    handlePosition: Offset,
    anchor: Position,
    baseSelection: Selection,
  ): Boolean {
    if (activeDrag) {
      return false
    }
    pendingContext = null
    dragContext =
      EditorSelectionHandleDragContext(
        type = type,
        startTouchPosition = touchPosition,
        startHandlePosition = handlePosition,
        anchor = anchor,
        baseSelection = baseSelection,
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
      baseSelection = drag.baseSelection,
      context = context,
    )
    val point = context.geometry.resolvePoint(positionInNode = selectionPosition) ?: return false
    if (point.page < 0) {
      return false
    }
    return context.editor.dispatchSelectionHandleExtension(
      point = point,
      anchor = drag.anchor,
      baseSelection = drag.baseSelection,
    )
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
  val anchor: Position,
  val baseSelection: Selection?,
)
