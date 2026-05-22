package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp

internal data class EditorSelectionExtensionContext(
  val anchor: PageRect,
  val initialSelection: Selection,
)

internal class EditorSelectionExpansionSemantic {
  private var context: EditorSelectionExtensionContext? = null
  private var awaitingWordSelectionCommit = false

  fun reset() {
    context = null
    awaitingWordSelectionCommit = false
  }

  val isAwaitingWordSelectionCommit: Boolean
    get() = awaitingWordSelectionCommit

  fun awaitWordSelectionCommit() {
    context = null
    awaitingWordSelectionCommit = true
  }

  fun markWordSelectionCommitted() {
    awaitingWordSelectionCommit = false
  }

  fun context(editor: Editor): EditorSelectionExtensionContext? {
    val current = context
    if (current != null) {
      return current
    }
    if (awaitingWordSelectionCommit) {
      return null
    }
    val resolved = editor.resolveSelectionExtensionContext() ?: return null
    context = resolved
    return resolved
  }
}

internal fun Editor.resolveSelectionExtensionContext(): EditorSelectionExtensionContext? {
  val initialSelection = state.selection ?: return null
  if (initialSelection.isCollapsed()) {
    return null
  }
  val anchor = selectionEndpoints()?.from ?: return null
  return EditorSelectionExtensionContext(anchor = anchor, initialSelection = initialSelection)
}

internal fun Editor.dispatchSelectionExtension(
  point: PagePoint,
  context: EditorSelectionExtensionContext,
): Boolean {
  if (point.page < 0) {
    return false
  }
  val anchor = context.anchor
  enqueue(
    Message.Selection(
      SelectionOp.ExtendTo(
        anchorPage = anchor.pageIdx,
        anchorX = anchor.rect.x,
        anchorY = anchor.rect.y + anchor.rect.height / 2f,
        headPage = point.page,
        headX = point.x,
        headY = point.y,
        initialSelection = context.initialSelection,
      )
    )
  )
  return true
}

private fun Selection?.isCollapsed(): Boolean = this == null || anchor == head
