package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorInteractionEffects

internal class EditorPointSelectionSemantic(private val effects: EditorInteractionEffects) {
  fun launchCursorMove(
    editor: Editor,
    point: PagePoint,
    beforeCommit: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(dispatchCursorMove(editor = editor, point = point, beforeCommit = beforeCommit))
    }
  }

  fun launchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    beforeCommit: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(
        dispatchSelectionExtension(editor = editor, point = point, beforeCommit = beforeCommit)
      )
    }
  }

  fun launchUnitSelection(
    editor: Editor,
    point: PagePoint,
    unit: SelectionPointUnit,
    beforeCommit: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      afterDispatch(
        dispatchUnitSelection(
          editor = editor,
          point = point,
          unit = unit,
          beforeCommit = beforeCommit,
        )
      )
    }
  }

  suspend fun dispatchCursorMove(
    editor: Editor,
    point: PagePoint,
    beforeCommit: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = SelectionOp.SetAt(page = point.page, x = point.x, y = point.y),
      beforeCommit = beforeCommit,
    )

  suspend fun dispatchSelectionExtension(
    editor: Editor,
    point: PagePoint,
    beforeCommit: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = point.selectionExtensionOp(currentSelection = editor.state.selection),
      beforeCommit = beforeCommit,
    )

  suspend fun dispatchUnitSelection(
    editor: Editor,
    point: PagePoint,
    unit: SelectionPointUnit,
    beforeCommit: ((EditorState) -> Unit)? = null,
  ): Boolean =
    dispatchSelection(
      editor = editor,
      op = SelectionOp.SelectUnitAt(page = point.page, x = point.x, y = point.y, unit = unit),
      beforeCommit = beforeCommit,
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
    beforeCommit: ((EditorState) -> Unit)?,
  ): Boolean {
    editor.await(beforeCommit = beforeCommit) { enqueue(Message.Selection(op)) }
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
