package co.typie.editor.runtime

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class EditorContextMenuStateTest {
  @Test
  fun `applied target cancels for another editor and otherwise matches version and selection once`() =
    runTest(StandardTestDispatcher()) {
      val dispatcher = StandardTestDispatcher(testScheduler)
      val editor = Editor(FakeFfiEditor(), this, dispatcher)
      val oldEditor = Editor(FakeFfiEditor(), this, dispatcher)
      val expectedSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 4, Affinity.Downstream),
        )
      val otherSelection =
        Selection(
          anchor = Position("text", 1, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val targetState = EditorState.Initial.copy(version = 5L, selection = expectedSelection)
      val state = EditorContextMenuState()

      state.requestShowForAppliedSelection(editor = editor, state = targetState)
      state.onEditorStateChanged(
        editor = oldEditor,
        state = targetState.copy(selection = otherSelection),
      )
      state.onEditorStateChanged(editor = editor, state = targetState)

      assertFalse(state.visible)

      state.requestShowForAppliedSelection(editor = editor, state = targetState)
      state.onEditorStateChanged(editor = editor, state = targetState.copy(version = 4L))

      assertFalse(state.visible)

      state.onEditorStateChanged(editor = editor, state = targetState)

      assertTrue(state.visible)

      state.hide()
      state.requestShowForAppliedSelection(editor = editor, state = targetState)
      state.onEditorStateChanged(
        editor = editor,
        state = targetState.copy(selection = otherSelection),
      )
      state.onEditorStateChanged(editor = editor, state = targetState.copy(version = 6L))

      assertFalse(state.visible)

      state.requestShowForAppliedSelection(editor = editor, state = targetState)
      state.reset()
      state.onEditorStateChanged(editor = editor, state = targetState)

      assertFalse(state.visible)
    }
}
