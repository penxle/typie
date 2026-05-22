package co.typie.editor.gesture

import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PlainRootNode
import co.typie.editor.ffi.PointerEvent as EditorPointerEvent
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorPrimaryTapDispatchTest {
  @Test
  fun `same collapsed cursor suppresses single tap reveal for context menu slot`() {
    val cursor = cursorAt(x = 10f)

    assertFalse(
      shouldRequestSingleTapBringIntoView(
        previousCursor = cursor,
        nextState = editorState(cursor = cursor),
      )
    )
  }

  @Test
  fun `changed collapsed cursor requests single tap reveal`() {
    assertTrue(
      shouldRequestSingleTapBringIntoView(
        previousCursor = cursorAt(x = 10f),
        nextState = editorState(cursor = cursorAt(x = 20f)),
      )
    )
  }

  @Test
  fun `primary tap dispatch sends down up and attaches reveal to committed version`() =
    runTest(StandardTestDispatcher()) {
      val requests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor(cursorProvider = { cursorAt(x = 20f) })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))

      assertTrue(
        editor.dispatchPrimaryTap(
          bringIntoViewRequests = requests,
          point = PagePoint(page = 0, x = 10f, y = 20f),
          clickCount = 1,
          previousCursor = cursorAt(x = 10f),
        )
      )

      val expectedMessages: List<Message> =
        listOf(
          Message.Pointer(EditorPointerEvent.Down(page = 0, x = 10f, y = 20f, count = 1)),
          Message.Pointer(EditorPointerEvent.Up),
        )
      assertEquals(expectedMessages, fake.enqueued)
      assertEquals(
        EditorBringIntoViewTarget.CurrentCursorLine,
        requests.activateForVersion(version = 1L),
      )
    }

  @Test
  fun `primary tap dispatch is skipped when interaction core blocks page`() =
    runTest(StandardTestDispatcher()) {
      val requests = EditorBringIntoViewRequests()
      val fake = FakeFfiEditor(cursorProvider = { cursorAt(x = 20f) })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))

      assertFalse(
        editor.dispatchPrimaryTap(
          bringIntoViewRequests = requests,
          point = PagePoint(page = -1, x = 10f, y = 20f),
          clickCount = 1,
          previousCursor = cursorAt(x = 10f),
        )
      )

      assertEquals(emptyList(), fake.enqueued)
      assertNull(requests.activateForVersion(version = 1L))
    }

  private fun cursorAt(x: Float): CursorMetrics =
    CursorMetrics(
      pageIdx = 0,
      caret = Rect(x = x, y = 0f, width = 1f, height = 12f),
      line = Rect(x = 0f, y = 0f, width = 100f, height = 12f),
    )

  private fun editorState(cursor: CursorMetrics?): EditorState =
    EditorState(
      version = 1L,
      cursor = cursor,
      selection = Selection(anchor = Position("a", 0), head = Position("a", 0)),
      pageSizes = emptyList(),
      externalElements = emptyList(),
      rootAttrs = PlainRootNode(layoutMode = LayoutMode.Continuous(maxWidth = 0)),
      rootModifiers = emptyList(),
      ime = Ime(text = "", windowStart = 0, selection = ImeRange(0, 0), composing = null),
    )
}
