package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
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
  private var wordSelectionBaseline: Selection? = null
  private var wordSelectionCommitMarked = false

  fun reset() {
    context = null
    awaitingWordSelectionCommit = false
    wordSelectionBaseline = null
    wordSelectionCommitMarked = false
  }

  val isAwaitingWordSelectionCommit: Boolean
    get() = awaitingWordSelectionCommit

  fun awaitWordSelectionCommit(baselineSelection: Selection? = null) {
    context = null
    awaitingWordSelectionCommit = true
    wordSelectionBaseline = baselineSelection
    wordSelectionCommitMarked = false
  }

  fun markWordSelectionCommitted() {
    wordSelectionCommitMarked = true
  }

  fun context(editor: Editor): EditorSelectionExtensionContext? {
    val current = context
    if (current != null) {
      return current
    }
    if (awaitingWordSelectionCommit) {
      if (!wordSelectionCommitMarked) {
        return null
      }
      return adoptWordSelection(editor)
    }
    val resolved = editor.resolveSelectionExtensionContext() ?: return null
    context = resolved
    return resolved
  }

  private fun adoptWordSelection(editor: Editor): EditorSelectionExtensionContext? {
    val selection = editor.state.selection ?: return null
    if (selection.isCollapsed()) {
      return null
    }
    if (wordSelectionBaseline != null && selection == wordSelectionBaseline) {
      return null
    }

    val resolved = editor.resolveSelectionExtensionContext() ?: return null
    context = resolved
    awaitingWordSelectionCommit = false
    wordSelectionBaseline = null
    wordSelectionCommitMarked = false
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
