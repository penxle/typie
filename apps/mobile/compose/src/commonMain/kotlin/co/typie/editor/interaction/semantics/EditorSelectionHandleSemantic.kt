package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp

internal class EditorSelectionHandleSemantic(
  private val pointSelection: EditorPointSelectionSemantic,
  private val contextMenu: EditorContextMenuSemantic,
) {
  private var pendingContextMenuRequest: PendingContextMenuRequest? = null

  fun enqueueExtension(
    editor: Editor,
    point: PagePoint,
    anchor: Position,
    baseSelection: Selection? = null,
  ): SelectionOp.ExtendTo? {
    val op =
      point.selectionHandleExtensionOp(anchor = anchor, baseSelection = baseSelection)
        ?: return null
    editor.enqueue(Message.Selection(op))
    return op
  }

  fun requestContextMenuAfterSelection(editor: Editor, terminalExtension: SelectionOp.ExtendTo?) {
    cancelPendingContextMenuRequest()
    if (terminalExtension == null) {
      contextMenu.requestShowForAppliedSelection(editor = editor, state = editor.appliedState)
      return
    }

    val request = PendingContextMenuRequest(editor = editor)
    pendingContextMenuRequest = request
    pointSelection.launchSelection(
      editor = request.editor,
      op = terminalExtension,
      onApplied = { snapshot ->
        if (pendingContextMenuRequest === request) {
          pendingContextMenuRequest = null
          contextMenu.requestShowForAppliedSelection(editor = request.editor, state = snapshot)
        }
      },
      afterDispatch = { dispatched ->
        if (!dispatched && pendingContextMenuRequest === request) {
          pendingContextMenuRequest = null
        }
      },
    )
  }

  fun cancelPendingContextMenuRequest() {
    pendingContextMenuRequest = null
  }

  fun reset() {
    cancelPendingContextMenuRequest()
  }

  private data class PendingContextMenuRequest(val editor: Editor)
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
