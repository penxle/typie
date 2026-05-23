package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.ffi.Rect
import co.typie.editor.interaction.EditorEdgeAutoScrollViewport
import co.typie.editor.interaction.EditorInteractionEffects
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
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
      val semantic = EditorCursorMoveSemantic(effects = UnusedEffects)
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
      val semantic = EditorCursorMoveSemantic(effects = UnusedEffects)
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
      val semantic = EditorCursorMoveSemantic(effects = UnusedEffects)

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

  @Test
  fun `cursor hit delegates to engine result instead of caret rectangle`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 20f) },
          cursorHitProvider = { page, x, y -> page == 0 && x == 99f && y == 99f },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val semantic = EditorCursorMoveSemantic(effects = UnusedEffects)

      assertFalse(
        semantic.isCursorHit(editor = editor, point = PagePoint(page = 0, x = 20f, y = 6f))
      )
    }

  private fun cursorAt(x: Float): CursorMetrics =
    CursorMetrics(
      pageIdx = 0,
      caret = Rect(x = x, y = 0f, width = 1f, height = 12f),
      line = Rect(x = 0f, y = 0f, width = 100f, height = 12f),
    )

  private object UnusedEffects : EditorInteractionEffects {
    override fun resolvePoint(positionInNode: Offset): PagePoint? =
      error("Unused in direct cursor semantic dispatch tests")

    override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? =
      error("Unused in direct cursor semantic dispatch tests")

    override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? =
      error("Unused in direct cursor semantic dispatch tests")

    override fun dispatchEdgeAutoScroll(delta: Offset): Offset =
      error("Unused in direct cursor semantic dispatch tests")

    override fun scheduleTapDispatch(dispatchAtMillis: Long) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun cancelTapDispatch() = error("Unused in direct cursor semantic dispatch tests")

    override fun scheduleLongPressDispatch(
      pointerId: Long,
      position: Offset,
      dispatchAtMillis: Long,
    ) = error("Unused in direct cursor semantic dispatch tests")

    override fun cancelLongPressDispatch() =
      error("Unused in direct cursor semantic dispatch tests")

    override fun launchInteraction(block: suspend () -> Unit) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun requestFocus(editor: Editor): Boolean =
      error("Unused in direct cursor semantic dispatch tests")

    override fun enqueuePointerCancel() = error("Unused in direct cursor semantic dispatch tests")

    override fun setScrollGestureLocked(locked: Boolean) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun performSelectionHaptic() = error("Unused in direct cursor semantic dispatch tests")

    override fun requestCurrentCursorLine(version: Long) =
      error("Unused in direct cursor semantic dispatch tests")
  }
}
