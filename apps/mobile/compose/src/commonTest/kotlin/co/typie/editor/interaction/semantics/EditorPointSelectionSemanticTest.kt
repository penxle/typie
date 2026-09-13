package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.CommandRejection
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorInteractionEffects
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorPointSelectionSemanticTest {
  @Test
  fun `pointer selection lets Android IME finish composition and commits before moving elsewhere`() =
    runTest(StandardTestDispatcher()) {
      for (platform in Platform.entries) {
        for (path in listOf("immediate", "suspending", "enqueued")) {
          val fake =
            FakeFfiEditor(
              imeProvider = { _, _ ->
                Ime(
                  text = "日本語 text",
                  windowStart = 0,
                  selection = ImeRange(3, 3),
                  composing = ImeRange(0, 3),
                )
              }
            )
          val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
          editor.setImeSessionActive(true)
          fake.applySnapshot(editor)
          val semantic = EditorPointSelectionSemantic(effects = UnusedEffects, platform = platform)
          val point = PagePoint(page = 0, x = 100f, y = 20f)
          val selection = SelectionOp.SetAt(page = 0, x = 100f, y = 20f)

          when (path) {
            "immediate" -> semantic.applySelection(editor, selection)
            "suspending" -> semantic.dispatchCursorMove(editor, point)
            "enqueued" -> semantic.enqueueCursorMove(editor, point)
          }
          testScheduler.runCurrent()

          assertEquals(
            if (platform == Platform.Android) {
              listOf(Message.Selection(selection))
            } else {
              listOf(Message.TextInput(listOf(FlatImeOp.CommitAsIs)), Message.Selection(selection))
            },
            fake.enqueued,
            "$platform/$path",
          )
        }
      }
    }

  @Test
  fun `cursor move sends set at and runs before commit hook`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor(cursorProvider = { cursorAt(x = 20f) })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val semantic =
        EditorPointSelectionSemantic(effects = UnusedEffects, platform = Platform.Desktop)
      var onAppliedCalled = false

      assertTrue(
        semantic.dispatchCursorMove(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          onApplied = { onAppliedCalled = true },
        )
      )

      val expectedMessages: List<Message> =
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)))
      assertEquals(expectedMessages, fake.enqueued)
      assertTrue(onAppliedCalled)
    }

  @Test
  fun `cursor move semantic does not own selection hit admission`() =
    runTest(StandardTestDispatcher()) {
      val semantic =
        EditorPointSelectionSemantic(effects = UnusedEffects, platform = Platform.Desktop)
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 20f) },
          selectionProvider = { rangeSelection },
          selectionHitRectsProvider = {
            listOf(PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 0f)))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      fake.publishSnapshot(editor)
      fake.enqueued.clear()

      assertTrue(
        semantic.dispatchCursorMove(editor = editor, point = PagePoint(page = 0, x = 10f, y = 20f))
      )

      val expectedMessages: List<Message> =
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)))
      assertEquals(expectedMessages, fake.enqueued)
    }

  @Test
  fun `rejected cursor move does not run applied callback`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(cursorProvider = { cursorAt(x = 20f) }).apply {
          commandOutcomesProvider = { _, messages ->
            List(messages.size) { CommandOutcome.Rejected(CommandRejection.InvalidArgument) }
          }
        }
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val semantic =
        EditorPointSelectionSemantic(effects = UnusedEffects, platform = Platform.Desktop)
      var onAppliedCalled = false

      assertFalse(
        semantic.dispatchCursorMove(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          onApplied = { onAppliedCalled = true },
        )
      )
      assertFalse(onAppliedCalled)
    }

  @Test
  fun `selection extension uses the current selection anchor`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 1, Affinity.Downstream),
          head = Position("text", 3, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(cursorProvider = { cursorAt(x = 20f) }, selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      fake.publishSnapshot(editor)
      val semantic =
        EditorPointSelectionSemantic(effects = UnusedEffects, platform = Platform.Desktop)

      assertTrue(
        semantic.dispatchSelectionExtension(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Selection(
            SelectionOp.ExtendTo(
              anchor = selection.anchor,
              headPage = 0,
              headX = 10f,
              headY = 20f,
              baseSelection = null,
              allowCollapse = true,
            )
          )
        )
      assertEquals(expectedMessages, fake.enqueued)
    }

  @Test
  fun `unit selection is not suppressed by selection hit guard`() =
    runTest(StandardTestDispatcher()) {
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 20f) },
          selectionProvider = { rangeSelection },
          selectionHitRectsProvider = { FakeFfiEditor.coveringHitRects(0) },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      fake.publishSnapshot(editor)
      fake.enqueued.clear()
      val semantic =
        EditorPointSelectionSemantic(effects = UnusedEffects, platform = Platform.Desktop)

      assertTrue(
        semantic.dispatchUnitSelection(
          editor = editor,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          unit = SelectionPointUnit.Word,
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Selection(
            SelectionOp.SelectUnitAt(page = 0, x = 10f, y = 20f, unit = SelectionPointUnit.Word)
          )
        )
      assertEquals(expectedMessages, fake.enqueued)
    }

  private fun cursorAt(x: Float): CursorMetrics =
    CursorMetrics(
      pageIdx = 0,
      caret = Rect(x = x, y = 0f, width = 1f, height = 12f),
      line = Rect(x = 0f, y = 0f, width = 100f, height = 12f),
    )

  private object UnusedEffects : EditorInteractionEffects {
    override fun dispatchEdgeAutoScroll(delta: Offset): Offset =
      error("Unused in direct cursor semantic dispatch tests")

    override fun scheduleTapDispatch(dispatchAtMillis: Long) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun cancelTapDispatch() = error("Unused in direct cursor semantic dispatch tests")

    override fun scheduleTapSequenceConfirmation(onConfirmed: () -> Unit) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun cancelTapSequenceConfirmation() =
      error("Unused in direct cursor semantic dispatch tests")

    override fun scheduleLongPressDispatch(
      pointerId: Long,
      position: Offset,
      dispatchAtMillis: Long,
    ) = error("Unused in direct cursor semantic dispatch tests")

    override fun cancelLongPressDispatch() =
      error("Unused in direct cursor semantic dispatch tests")

    override fun launchInteraction(block: suspend () -> Unit) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun requestEditing(editor: Editor): Boolean =
      error("Unused in direct cursor semantic dispatch tests")

    override fun showReadingTapHint() = error("Unused in direct cursor semantic dispatch tests")

    override fun requestFocus(editor: Editor): Boolean =
      error("Unused in direct cursor semantic dispatch tests")

    override fun requestSoftwareKeyboard() =
      error("Unused in direct cursor semantic dispatch tests")

    override fun setScrollGestureLocked(locked: Boolean) =
      error("Unused in direct cursor semantic dispatch tests")

    override fun performSelectionHaptic() = error("Unused in direct cursor semantic dispatch tests")

    override fun requestPointerSelectionHead(version: Long) =
      error("Unused in direct cursor semantic dispatch tests")
  }
}
