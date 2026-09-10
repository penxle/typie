package co.typie.editor.runtime

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class EditorContextMenuStateTest {
  @Test
  fun `pointer menu can open for a caret only after publication and dismissal cancels it`() =
    runTest {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val caret = Position("text", 2, Affinity.Downstream)
      val target = EditorState.Initial.copy(version = 5L, selection = Selection(caret, caret))
      val state = EditorContextMenuState()
      state.requestShowForAppliedSelection(editor, target)
      state.onEditorStateChanged(editor, target)
      assertFalse(state.visible)
      state.requestShowForAppliedSelection(editor, target, PagePoint(0, 20f, 40f))
      state.onEditorStateChanged(editor, target.copy(version = 4L))
      assertFalse(state.visible)
      state.onEditorStateChanged(editor, target)
      assertTrue(state.visible)
      assertEquals(EditorContextMenuMode.Expanded, state.mode)
      assertEquals(PagePoint(0, 20f, 40f), state.pointerPosition)
      state.hide()
      assertNull(state.pointerPosition)
      state.requestShowForAppliedSelection(
        editor,
        target.copy(version = 6L),
        PagePoint(0, 30f, 40f),
      )
      state.hide()
      state.onEditorStateChanged(editor, target.copy(version = 6L))
      assertFalse(state.visible)
    }

  @Test
  fun `touch menu expands without changing its anchor and survives selection publication`() =
    runTest {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val selection =
        Selection(
          Position("text", 0, Affinity.Downstream),
          Position("text", 4, Affinity.Downstream),
        )
      val initial = EditorState.Initial.copy(version = 1L, selection = selection)
      val state = EditorContextMenuState()
      state.show(initial)
      assertEquals(EditorContextMenuMode.Compact, state.mode)
      state.expand()
      assertEquals(EditorContextMenuMode.Expanded, state.mode)
      assertNull(state.pointerPosition)

      val next =
        initial.copy(
          version = 2L,
          selection = selection.copy(head = Position("text", 8, Affinity.Downstream)),
        )
      state.requestShowForAppliedSelection(editor, next, mode = state.mode)
      state.onEditorStateChanged(editor, next)
      assertTrue(state.isVisibleFor(next))
      assertEquals(EditorContextMenuMode.Expanded, state.mode)
      assertNull(state.pointerPosition)

      state.hide()
      state.show(initial)
      assertEquals(EditorContextMenuMode.Compact, state.mode)
    }

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
