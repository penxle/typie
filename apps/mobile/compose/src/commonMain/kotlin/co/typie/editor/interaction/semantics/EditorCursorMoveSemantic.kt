package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorInteractionEffects

// FIXME: 이름과 책임이 이상함. 이름은 CursorMoveSemantic인데 SelectUnitAt까지 하고 있고, 제스처 쪽에 있어야 할
// clickCount/modifier에 의한 분기가 있음
internal class EditorCursorMoveSemantic(private val effects: EditorInteractionEffects) {
  fun reset() {}

  fun requestFocus(editor: Editor): Boolean = effects.requestFocus(editor)

  fun requestCurrentCursorLine(version: Long) {
    effects.requestCurrentCursorLine(version = version)
  }

  fun launchPrimaryClick(
    editor: Editor,
    point: PagePoint,
    clickCount: Int,
    inputModifiers: InputModifiers = InputModifiers(),
    beforeCommit: ((EditorState) -> Unit)? = null,
    afterDispatch: (Boolean) -> Unit = {},
  ) {
    effects.launchInteraction {
      val dispatched =
        dispatchPrimaryClick(
          editor = editor,
          point = point,
          clickCount = clickCount,
          inputModifiers = inputModifiers,
          beforeCommit = beforeCommit,
        )
      afterDispatch(dispatched)
    }
  }
}

internal suspend fun EditorCursorMoveSemantic.dispatchPrimaryClick(
  editor: Editor,
  point: PagePoint,
  clickCount: Int,
  inputModifiers: InputModifiers = InputModifiers(),
  beforeCommit: ((EditorState) -> Unit)? = null,
): Boolean {
  val currentSelection = editor.state.selection
  editor.await(beforeCommit = beforeCommit) {
    enqueue(
      point.toSelectionMessage(
        clickCount = clickCount,
        inputModifiers = inputModifiers,
        currentSelection = currentSelection,
      )
    )
  }
  return true
}

internal fun EditorCursorMoveSemantic.enqueuePrimaryClick(
  editor: Editor,
  point: PagePoint,
  clickCount: Int,
  inputModifiers: InputModifiers = InputModifiers(),
): Boolean {
  editor.enqueue(
    point.toSelectionMessage(
      clickCount = clickCount,
      inputModifiers = inputModifiers,
      currentSelection = editor.state.selection,
    )
  )
  return true
}

private fun PagePoint.toSelectionMessage(
  clickCount: Int,
  inputModifiers: InputModifiers,
  currentSelection: co.typie.editor.ffi.Selection?,
): Message.Selection {
  val op =
    when {
      clickCount <= 1 && inputModifiers.shift && currentSelection != null ->
        SelectionOp.ExtendTo(
          anchor = currentSelection.anchor,
          headPage = page,
          headX = x,
          headY = y,
          baseSelection = null,
        )
      clickCount <= 1 -> SelectionOp.SetAt(page = page, x = x, y = y)
      clickCount == 2 ->
        SelectionOp.SelectUnitAt(page = page, x = x, y = y, unit = SelectionPointUnit.Word)
      else ->
        SelectionOp.SelectUnitAt(page = page, x = x, y = y, unit = SelectionPointUnit.Paragraph)
    }
  return Message.Selection(op)
}
