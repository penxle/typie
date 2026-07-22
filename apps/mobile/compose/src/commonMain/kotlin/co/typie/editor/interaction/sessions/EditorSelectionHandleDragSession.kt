package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.contains
import co.typie.editor.interaction.gestures.EditorSelectionHandleTableCellHandoff
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
import co.typie.editor.interaction.resolveActiveTableCellSelection
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

  fun hasMovedPastSlop(
    type: EditorSelectionHandleType,
    touchPosition: Offset,
    dragSlop: Float,
  ): Boolean {
    val pending = pendingContext?.takeIf { it.type == type } ?: return false
    return (touchPosition - pending.touchPosition).getDistance() > dragSlop
  }

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

    val endpoints = context.editor.tickSelectionEndpoints ?: return false
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
        tableId = null,
      )
    return true
  }

  fun adoptDrag(
    type: EditorSelectionHandleType,
    touchPosition: Offset,
    handlePosition: Offset,
    tableId: String,
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
        tableId = tableId,
      )
    return true
  }

  fun tableCellHandoff(
    type: EditorSelectionHandleType,
    touchPosition: Offset,
    context: EditorGestureContext,
  ): EditorSelectionHandleTableCellHandoff? {
    val drag = dragContext ?: return null
    if (drag.type != type) {
      return null
    }
    val selectionPosition = drag.selectionPosition(touchPosition = touchPosition)
    val point = context.geometry.resolvePoint(positionInNode = selectionPosition) ?: return null
    if (point.page < 0) {
      return null
    }
    val activeSelection =
      resolveActiveTableCellSelection(context.editor)?.takeIf { selection ->
        drag.tableId == null || selection.overlay.tableId == drag.tableId
      } ?: return null
    if (
      activeSelection.range.rowStart == activeSelection.range.rowEnd &&
        activeSelection.range.colStart == activeSelection.range.colEnd
    ) {
      return null
    }
    if (
      !activeSelection.overlay.contains(
        point = point,
        layoutMode = context.editor.rootAttrs?.layoutMode,
        project = context.geometry::resolvePagePosition,
      )
    ) {
      return null
    }
    val baseSelection = drag.baseSelection ?: context.editor.selection ?: return null
    return EditorSelectionHandleTableCellHandoff(
      touchPosition = touchPosition,
      handlePosition = selectionPosition,
      tableId = activeSelection.overlay.tableId,
      anchor = baseSelection.anchor,
      baseSelection = baseSelection,
    )
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

    val selectionPosition = drag.selectionPosition(touchPosition = touchPosition)
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
  val tableId: String?,
) {
  fun selectionPosition(touchPosition: Offset): Offset =
    startHandlePosition + (touchPosition - startTouchPosition)
}
