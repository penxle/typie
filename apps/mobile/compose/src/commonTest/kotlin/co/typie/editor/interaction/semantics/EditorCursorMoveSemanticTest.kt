package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.ffi.Rect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorCursorMoveSemanticTest {
  @Test
  fun `primary click dispatch sends down up and runs before commit hook`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor(cursorProvider = { cursorAt(x = 20f) })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val semantic = EditorCursorMoveSemantic()
      var beforeCommitCalled = false

      assertTrue(
        semantic.dispatchPrimaryClick(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          clickCount = 1,
          beforeCommit = { beforeCommitCalled = true },
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Pointer(EditorPointerEvent.Down(page = 0, x = 10f, y = 20f, count = 1)),
          Message.Pointer(EditorPointerEvent.Up),
        )
      assertEquals(expectedMessages, fake.enqueued)
      assertTrue(beforeCommitCalled)
    }

  @Test
  fun `primary click semantic does not own selection hit admission`() =
    runTest(StandardTestDispatcher()) {
      val semantic = EditorCursorMoveSemantic()
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 20f) },
          selectionHitProvider = { page, x, y -> page == 0 && x == 10f && y == 20f },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))

      assertTrue(
        semantic.dispatchPrimaryClick(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          clickCount = 1,
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Pointer(EditorPointerEvent.Down(page = 0, x = 10f, y = 20f, count = 1)),
          Message.Pointer(EditorPointerEvent.Up),
        )
      assertEquals(expectedMessages, fake.enqueued)
    }

  @Test
  fun `double tap is not suppressed by selection hit guard`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 20f) },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val semantic = EditorCursorMoveSemantic()

      assertTrue(
        semantic.dispatchPrimaryClick(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          clickCount = 2,
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Pointer(EditorPointerEvent.Down(page = 0, x = 10f, y = 20f, count = 2)),
          Message.Pointer(EditorPointerEvent.Up),
        )
      assertEquals(expectedMessages, fake.enqueued)
    }

  private fun cursorAt(x: Float): CursorMetrics =
    CursorMetrics(
      pageIdx = 0,
      caret = Rect(x = x, y = 0f, width = 1f, height = 12f),
      line = Rect(x = 0f, y = 0f, width = 100f, height = 12f),
    )
}
