package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionOp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorSelectionExpansionSemanticTest {
  @Test
  fun `double tap drag extension dispatches extend to with materialized base selection`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
          fromPosition = Position("text", 0),
          toPosition = Position("text", 5),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}

      val context =
        editor.resolveSelectionExtensionContext() ?: error("selection context should materialize")
      assertTrue(
        editor.dispatchSelectionExtension(
          point = PagePoint(page = 1, x = 30f, y = 40f),
          context = context,
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Selection(
            SelectionOp.ExtendTo(
              anchor = selection.anchor,
              headPage = 1,
              headX = 30f,
              headY = 40f,
              baseSelection = selection,
              allowCollapse = false,
            )
          )
        )
      assertEquals(expectedMessages, fake.enqueued)
    }

  @Test
  fun `double tap drag extension materializes without selection endpoints`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { null })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}

      assertEquals(selection, editor.resolveSelectionExtensionContext()?.baseSelection)
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun `double tap drag extension keeps the initially materialized selection while dragging`() =
    runTest(StandardTestDispatcher()) {
      val baseSelection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val expandedSelection = Selection(anchor = Position("text", 0), head = Position("text", 11))
      var currentSelection = baseSelection
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
          fromPosition = Position("text", 0),
          toPosition = Position("text", 5),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { currentSelection },
          selectionEndpointsProvider = { endpoints },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val selectionExpansionSemantic = EditorSelectionExpansionSemantic()

      editor.sync {}
      assertTrue(
        editor.dispatchSelectionExtension(
          point = PagePoint(page = 0, x = 30f, y = 40f),
          context = selectionExpansionSemantic.context(editor)!!,
        )
      )

      currentSelection = expandedSelection
      fake.enqueued.clear()
      editor.sync {}

      assertTrue(
        editor.dispatchSelectionExtension(
          point = PagePoint(page = 0, x = 12f, y = 40f),
          context = selectionExpansionSemantic.context(editor)!!,
        )
      )

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(baseSelection, extend.baseSelection)
      assertFalse(extend.allowCollapse)
    }

  @Test
  fun `double tap drag context adopts the current range without a baseline wait`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
          fromPosition = Position("text", 0),
          toPosition = Position("text", 5),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val selectionExpansionSemantic = EditorSelectionExpansionSemantic()

      editor.sync {}

      assertEquals(selection, selectionExpansionSemantic.context(editor)?.baseSelection)
    }

  @Test
  fun `word selection commit gate prevents stale context before commit`() =
    runTest(StandardTestDispatcher()) {
      val staleSelection = Selection(anchor = Position("old", 0), head = Position("old", 5))
      val wordSelection = Selection(anchor = Position("word", 0), head = Position("word", 4))
      var currentSelection = staleSelection
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
          fromPosition = Position("text", 0),
          toPosition = Position("text", 5),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { currentSelection },
          selectionEndpointsProvider = { endpoints },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val selectionExpansionSemantic = EditorSelectionExpansionSemantic()

      editor.sync {}
      selectionExpansionSemantic.awaitWordSelectionCommit()

      assertNull(selectionExpansionSemantic.context(editor))

      currentSelection = wordSelection
      editor.sync {}
      selectionExpansionSemantic.markWordSelectionCommitted()

      assertEquals(wordSelection, selectionExpansionSemantic.context(editor)?.baseSelection)
    }

  @Test
  fun `word selection context waits until current selection differs from baseline`() =
    runTest(StandardTestDispatcher()) {
      val staleSelection = Selection(anchor = Position("old", 0), head = Position("old", 5))
      val wordSelection = Selection(anchor = Position("word", 0), head = Position("word", 4))
      var currentSelection = staleSelection
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
          fromPosition = Position("text", 0),
          toPosition = Position("text", 5),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { currentSelection },
          selectionEndpointsProvider = { endpoints },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val selectionExpansionSemantic = EditorSelectionExpansionSemantic()

      editor.sync {}
      selectionExpansionSemantic.awaitWordSelectionCommit(baselineSelection = staleSelection)
      selectionExpansionSemantic.markWordSelectionCommitted()

      assertNull(selectionExpansionSemantic.context(editor))

      currentSelection = wordSelection
      editor.sync {}

      assertEquals(wordSelection, selectionExpansionSemantic.context(editor)?.baseSelection)
    }
}
