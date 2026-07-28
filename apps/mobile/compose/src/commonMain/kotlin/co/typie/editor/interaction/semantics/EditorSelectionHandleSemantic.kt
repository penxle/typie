package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp

internal fun Editor.dispatchSelectionHandleExtension(
  point: PagePoint,
  anchor: Position,
  baseSelection: Selection? = null,
): Boolean {
  val op =
    point.selectionHandleExtensionOp(anchor = anchor, baseSelection = baseSelection) ?: return false
  enqueue(Message.Selection(op))
  return true
}

internal fun PagePoint.selectionHandleExtensionOp(
  anchor: Position,
  baseSelection: Selection? = null,
): SelectionOp.ExtendTo? {
  if (page < 0) {
    return null
  }
  return SelectionOp.ExtendTo(
    anchor = anchor,
    headPage = page,
    headX = x,
    headY = y,
    baseSelection = baseSelection,
    allowCollapse = false,
  )
}
