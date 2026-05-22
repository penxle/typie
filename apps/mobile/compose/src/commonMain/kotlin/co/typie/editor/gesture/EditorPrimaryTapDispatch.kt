package co.typie.editor.gesture

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent
import co.typie.editor.ffi.Selection
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget

internal suspend fun Editor.dispatchPrimaryTap(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  point: PagePoint,
  clickCount: Int,
  previousCursor: CursorMetrics?,
  runtime: EditorInteractionRuntimeRead = EditorInteractionRuntimeRead(),
): Boolean {
  if (
    !EditorInteractionCore()
      .decide(command = EditorInteractionCommand.TapDispatch(page = point.page), runtime = runtime)
  ) {
    return false
  }

  await(
    beforeCommit = { snapshot ->
      if (clickCount == 1 && shouldRequestSingleTapBringIntoView(previousCursor, snapshot)) {
        bringIntoViewRequests.requestForVersion(
          target = EditorBringIntoViewTarget.CurrentCursorLine,
          version = snapshot.version,
        )
      }
    }
  ) {
    enqueue(
      Message.Pointer(
        PointerEvent.Down(page = point.page, x = point.x, y = point.y, count = clickCount)
      )
    )
    enqueue(Message.Pointer(PointerEvent.Up))
  }
  return true
}

internal fun shouldRequestSingleTapBringIntoView(
  previousCursor: CursorMetrics?,
  nextState: EditorState,
): Boolean {
  val nextCursor = nextState.cursor ?: return false
  if (
    nextState.selection.isCollapsed() &&
      previousCursor != null &&
      nextCursor.isSamePosition(previousCursor)
  ) {
    // TODO(editor-parity): same-cursor single tap should open the context menu slot.
    return false
  }
  return true
}

private fun Selection?.isCollapsed(): Boolean = this == null || anchor == head

private fun CursorMetrics.isSamePosition(other: CursorMetrics): Boolean =
  pageIdx == other.pageIdx &&
    caret.x == other.caret.x &&
    caret.y == other.caret.y &&
    line.y == other.line.y
