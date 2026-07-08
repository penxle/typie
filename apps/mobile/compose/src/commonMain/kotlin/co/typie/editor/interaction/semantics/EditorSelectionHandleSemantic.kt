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
  if (point.page < 0) {
    return false
  }
  enqueue(
    Message.Selection(
      SelectionOp.ExtendTo(
        anchor = anchor,
        headPage = point.page,
        headX = point.x,
        headY = point.y,
        baseSelection = baseSelection,
        allowCollapse = false,
      )
    )
  )
  return true
}
