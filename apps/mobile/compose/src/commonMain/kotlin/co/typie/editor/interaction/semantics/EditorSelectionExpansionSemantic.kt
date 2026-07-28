package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp

internal data class EditorSelectionExtensionContext(
  val anchor: Position,
  val baseSelection: Selection?,
)

internal class EditorSelectionExpansionSemantic {
  private var context: EditorSelectionExtensionContext? = null
  private var awaitingWordSelectionApplied = false
  private var wordSelectionBaseline: Selection? = null
  private var wordSelectionAppliedMarked = false

  fun reset() {
    context = null
    awaitingWordSelectionApplied = false
    wordSelectionBaseline = null
    wordSelectionAppliedMarked = false
  }

  val isAwaitingWordSelectionApplied: Boolean
    get() = awaitingWordSelectionApplied

  fun awaitWordSelectionApplied(baselineSelection: Selection? = null) {
    context = null
    awaitingWordSelectionApplied = true
    wordSelectionBaseline = baselineSelection
    wordSelectionAppliedMarked = false
  }

  fun markWordSelectionApplied() {
    wordSelectionAppliedMarked = true
  }

  fun context(editor: Editor): EditorSelectionExtensionContext? {
    val current = context
    if (current != null) {
      return current
    }
    if (awaitingWordSelectionApplied) {
      if (!wordSelectionAppliedMarked) {
        return null
      }
      return adoptWordSelection(editor)
    }
    val resolved = editor.resolveSelectionExtensionContext() ?: return null
    context = resolved
    return resolved
  }

  private fun adoptWordSelection(editor: Editor): EditorSelectionExtensionContext? {
    val selection = editor.appliedState.selection ?: return null
    if (selection.isCollapsed()) {
      return null
    }
    if (wordSelectionBaseline != null && selection == wordSelectionBaseline) {
      return null
    }

    val resolved = editor.resolveSelectionExtensionContext() ?: return null
    context = resolved
    awaitingWordSelectionApplied = false
    wordSelectionBaseline = null
    wordSelectionAppliedMarked = false
    return resolved
  }
}

internal fun Editor.resolveSelectionExtensionContext(): EditorSelectionExtensionContext? {
  val baseSelection = appliedState.selection ?: return null
  if (baseSelection.isCollapsed()) {
    return null
  }
  return EditorSelectionExtensionContext(
    anchor = baseSelection.anchor,
    baseSelection = baseSelection,
  )
}

internal fun Editor.dispatchSelectionExtension(
  point: PagePoint,
  context: EditorSelectionExtensionContext,
): Boolean {
  val op = point.selectionExtensionOp(context = context) ?: return false
  enqueue(Message.Selection(op))
  return true
}

internal fun PagePoint.selectionExtensionOp(
  context: EditorSelectionExtensionContext
): SelectionOp.ExtendTo? {
  if (page < 0) {
    return null
  }
  return SelectionOp.ExtendTo(
    anchor = context.anchor,
    headPage = page,
    headX = x,
    headY = y,
    baseSelection = context.baseSelection,
    allowCollapse = false,
  )
}
