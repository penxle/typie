package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp

internal class EditorTableHandleDragSession {
  private var pendingContext: EditorPendingTableHandleDrag? = null
  private var dragContext: EditorTableHandleDragContext? = null

  val pendingDrag: Boolean
    get() = pendingContext != null

  val activeDrag: Boolean
    get() = dragContext != null

  val dragging: Boolean
    get() = pendingDrag || activeDrag

  val activeContext: EditorTableHandleDragContext?
    get() = dragContext

  fun hasMovedPastSlop(touchPosition: Offset, dragSlop: Float): Boolean {
    val pending = pendingContext ?: return false
    return (touchPosition - pending.touchPosition).getDistance() > dragSlop
  }

  fun beginPending(touchPosition: Offset): Boolean {
    if (activeDrag) {
      return false
    }
    pendingContext = EditorPendingTableHandleDrag(touchPosition = touchPosition)
    return true
  }

  fun beginDrag(
    touchPosition: Offset,
    handleCenter: Offset,
    tableId: String,
    anchor: Position,
    baseSelection: Selection,
  ): Boolean {
    if (activeDrag) {
      return false
    }
    val startTouchPosition = pendingContext?.touchPosition ?: touchPosition
    pendingContext = null
    dragContext =
      EditorTableHandleDragContext(
        tableId = tableId,
        startTouchPosition = startTouchPosition,
        startHandlePosition = handleCenter,
        anchor = anchor,
        baseSelection = baseSelection,
      )
    return true
  }

  fun adoptDrag(
    touchPosition: Offset,
    handleCenter: Offset,
    tableId: String,
    anchor: Position,
    baseSelection: Selection,
  ): Boolean {
    if (activeDrag) {
      return false
    }
    pendingContext = null
    dragContext =
      EditorTableHandleDragContext(
        tableId = tableId,
        startTouchPosition = touchPosition,
        startHandlePosition = handleCenter,
        anchor = anchor,
        baseSelection = baseSelection,
      )
    return true
  }

  fun selectionPosition(touchPosition: Offset): Offset? {
    val drag = dragContext ?: return null
    return drag.startHandlePosition + (touchPosition - drag.startTouchPosition)
  }

  fun finish(): SelectionOp.ExtendTo? {
    val terminalExtension = dragContext?.latestExtension
    reset()
    return terminalExtension
  }

  fun reset() {
    pendingContext = null
    dragContext = null
  }
}

private data class EditorPendingTableHandleDrag(val touchPosition: Offset)

internal data class EditorTableHandleDragContext(
  val tableId: String,
  val startTouchPosition: Offset,
  val startHandlePosition: Offset,
  val anchor: Position,
  val baseSelection: Selection,
  var latestExtension: SelectionOp.ExtendTo? = null,
)
