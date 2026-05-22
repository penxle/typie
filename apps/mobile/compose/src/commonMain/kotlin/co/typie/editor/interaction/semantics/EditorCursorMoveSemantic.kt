package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent
import co.typie.editor.ffi.Selection

internal class EditorCursorMoveSemantic {
  fun reset() {}
}

internal suspend fun EditorCursorMoveSemantic.dispatchPrimaryClick(
  editor: Editor,
  point: PagePoint,
  clickCount: Int,
  beforeCommit: ((EditorState) -> Unit)? = null,
): Boolean {
  editor.await(beforeCommit = beforeCommit) {
    enqueue(
      Message.Pointer(
        PointerEvent.Down(page = point.page, x = point.x, y = point.y, count = clickCount)
      )
    )
    enqueue(Message.Pointer(PointerEvent.Up))
  }
  return true
}

internal fun EditorCursorMoveSemantic.isSelectionHit(editor: Editor, point: PagePoint): Boolean {
  val hit = editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (hit) {
    // TODO(editor-parity): open the selection context menu here once that state is hosted in the
    // interaction runtime.
  }
  return hit
}

internal fun EditorCursorMoveSemantic.hasRangeSelection(editor: Editor): Boolean =
  !editor.state.selection.isCollapsed()

private fun Selection?.isCollapsed(): Boolean = this == null || anchor == head
