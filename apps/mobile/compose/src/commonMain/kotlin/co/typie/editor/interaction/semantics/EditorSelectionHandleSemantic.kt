package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.SelectionOp

internal fun Editor.dispatchSelectionHandleExtension(point: PagePoint, anchor: PageRect): Boolean {
  if (point.page < 0) {
    return false
  }
  enqueue(
    Message.Selection(
      SelectionOp.ExtendTo(
        anchorPage = anchor.pageIdx,
        anchorX = anchor.rect.x,
        anchorY = anchor.rect.y + anchor.rect.height / 2f,
        headPage = point.page,
        headX = point.x,
        headY = point.y,
        initialSelection = null,
      )
    )
  )
  return true
}
