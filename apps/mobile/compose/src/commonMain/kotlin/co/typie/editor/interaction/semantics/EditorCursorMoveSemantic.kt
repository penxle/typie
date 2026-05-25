package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent
import co.typie.editor.interaction.EditorInteractionEffects

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
  editor.await(beforeCommit = beforeCommit) {
    enqueue(
      Message.Pointer(
        PointerEvent.Down(
          page = point.page,
          x = point.x,
          y = point.y,
          count = clickCount,
          modifiers = inputModifiers,
        )
      )
    )
    enqueue(Message.Pointer(PointerEvent.Up))
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
    Message.Pointer(
      PointerEvent.Down(
        page = point.page,
        x = point.x,
        y = point.y,
        count = clickCount,
        modifiers = inputModifiers,
      )
    )
  )
  editor.enqueue(Message.Pointer(PointerEvent.Up))
  return true
}
