package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorInteractionEffects

internal class EditorPointSelectionSemantic(private val effects: EditorInteractionEffects) {
  fun launchSelection(
    editor: Editor,
    op: SelectionOp,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(dispatchSelection(editor = editor, op = op, onApplied = onApplied))
    }
  }

  fun launchCursorMove(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(dispatchCursorMove(editor = editor, point = point, onApplied = onApplied))
    }
  }

  fun launchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(
        dispatchSelectionExtension(editor = editor, point = point, onApplied = onApplied)
      )
    }
  }

  fun launchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    context: EditorSelectionExtensionContext,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    val op =
      point.selectionExtensionOp(context = context)
        ?: run {
          afterDispatch(false)
          return
        }
    launchSelection(editor = editor, op = op, onApplied = onApplied, afterDispatch = afterDispatch)
  }

  fun launchUnitSelection(
    editor: Editor,
    point: PagePoint,
    unit: SelectionPointUnit,
    onApplied: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(
        dispatchUnitSelection(editor = editor, point = point, unit = unit, onApplied = onApplied)
      )
    }
  }

  suspend fun dispatchCursorMove(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = SelectionOp.SetAt(page = point.page, x = point.x, y = point.y),
      onApplied = onApplied,
    )

  suspend fun dispatchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    onApplied: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = point.selectionExtensionOp(currentSelection = editor.appliedState.selection),
      onApplied = onApplied,
    )

  suspend fun dispatchUnitSelection(
    editor: Editor,
    point: PagePoint,
    unit: SelectionPointUnit,
    onApplied: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = SelectionOp.SelectUnitAt(page = point.page, x = point.x, y = point.y, unit = unit),
      onApplied = onApplied,
    )

  fun enqueueCursorMove(editor: Editor, point: PagePoint): Boolean {
    editor.enqueue(
      Message.Selection(SelectionOp.SetAt(page = point.page, x = point.x, y = point.y))
    )
    return true
  }

  private suspend fun dispatchSelection(
    editor: Editor,
    op: SelectionOp,
    onApplied: ((EditorState) -> Unit)?,
  ): Boolean {
    val update = editor.update { enqueue(Message.Selection(op)) } ?: return false
    if (update.commandOutcomes.any { it is CommandOutcome.Rejected }) return false
    onApplied?.invoke(update.snapshot)
    return true
  }
}

private fun PagePoint.selectionExtensionOp(currentSelection: Selection?): SelectionOp =
  currentSelection?.let { selection ->
    SelectionOp.ExtendTo(
      anchor = selection.anchor,
      headPage = page,
      headX = x,
      headY = y,
      baseSelection = null,
      allowCollapse = true,
    )
  } ?: SelectionOp.SetAt(page = page, x = x, y = y)
