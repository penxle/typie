package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
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
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorInteractionControllerTest {
  @Test
  fun `pinch start cancels active pending tap stream`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)

      controller.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 0L)

      controller.applyEvent(EditorInteractionEvent.PinchStart)

      assertEquals(EditorInteractionMode.Pinching, controller.interactionMode)
      assertFalse(controller.hasActivePointer)
      assertEquals(1, host.cancelTapDispatchCount)
      assertEquals(1, host.pointerCancelCount)
    }

  @Test
  fun `tap rejected by page admission does not advance consecutive tap history`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      host.point = PagePoint(page = -1, x = 10f, y = 20f)
      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)

      host.point = PagePoint(page = 0, x = 10f, y = 20f)

      assertFalse(controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L))
      assertEquals(250L + 120L, host.scheduledTapDispatchAtMillis)
    }

  @Test
  fun `plain drag past tap slop cancels tap timer without taking over selection`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)

      assertFalse(controller.onPointerDown(pointerId = 1L, position = Offset.Zero, nowMillis = 0L))
      assertEquals(250L, host.scheduledTapDispatchAtMillis)

      assertFalse(
        controller.onPointerMove(pointerId = 1L, position = Offset(9f, 0f), nowMillis = 20L)
      )

      assertEquals(1, host.cancelTapDispatchCount)
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `pinch start clears pending double tap drag state`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)

      controller.applyEvent(EditorInteractionEvent.PinchStart)
      controller.applyEvent(EditorInteractionEvent.PinchEnd)

      assertFalse(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(5f, 0f),
          nowMillis = 140L,
        )
      )
      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
    }

  @Test
  fun `double tap drag extends selection directly from controller workflow`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L))
      advanceUntilIdle()

      assertTrue(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(5f, 0f),
          nowMillis = 140L,
        )
      )

      assertEquals(EditorInteractionMode.DoubleTapSelecting, controller.interactionMode)
      assertEquals(
        Message.Selection(
          SelectionOp.ExtendTo(
            anchorPage = 0,
            anchorX = 10f,
            anchorY = 24f,
            headPage = 0,
            headX = 15f,
            headY = 20f,
            initialSelection = selection,
          )
        ),
        fake.enqueued.last(),
      )
    }

  @Test
  fun `double tap drag keeps pending extension when pointer up beats word selection commit`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      controller.onPointerMove(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 140L)
      controller.onPointerUp(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 150L)
      advanceUntilIdle()

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(selection, extend.initialSelection)
      assertEquals(18f, extend.headX)
    }

  @Test
  fun `double tap drag can shrink back to the initial selected word range`() =
    runTest(StandardTestDispatcher()) {
      val initialSelection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val expandedSelection = Selection(anchor = Position("text", 0), head = Position("text", 12))
      var currentSelection = initialSelection
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { currentSelection },
          selectionEndpointsProvider = { endpoints },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      advanceUntilIdle()

      controller.onPointerMove(pointerId = 2L, position = start + Offset(12f, 0f), nowMillis = 140L)
      currentSelection = expandedSelection
      editor.sync {}
      fake.enqueued.clear()

      controller.onPointerMove(pointerId = 2L, position = start + Offset(5f, 0f), nowMillis = 150L)

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(initialSelection, extend.initialSelection)
      assertEquals(15f, extend.headX)
    }

  @Test
  fun `second pointer cancels pending double tap drag before it can extend selection`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      advanceUntilIdle()

      assertFalse(
        controller.onPointerDown(
          pointerId = 3L,
          position = start + Offset(1f, 0f),
          nowMillis = 130L,
        )
      )

      assertEquals(1, host.pointerCancelCount)
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(
        controller.onPointerMove(
          pointerId = 3L,
          position = start + Offset(12f, 0f),
          nowMillis = 140L,
        )
      )
    }

  @Test
  fun `second pointer cancel drops deferred double tap drag extension before word selection commit`() =
    runTest(StandardTestDispatcher()) {
      val selection = Selection(anchor = Position("text", 0), head = Position("text", 5))
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller = EditorInteractionController(editorProvider = { editor }, host = host)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      controller.onPointerMove(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 140L)

      assertFalse(controller.onPointerDown(pointerId = 3L, position = start, nowMillis = 150L))
      advanceUntilIdle()

      assertEquals(1, host.pointerCancelCount)
      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
    }

  private class TestHost(private val scope: TestScope) : EditorInteractionControllerHost {
    var scheduledTapDispatchAtMillis: Long? = null
    var cancelTapDispatchCount = 0
    var pointerCancelCount = 0
    var focused = false
    var point: PagePoint? = PagePoint(page = 0, x = 10f, y = 20f)
    val requestedBringIntoViewVersions = mutableListOf<Long>()

    override fun resolvePoint(positionInNode: Offset): PagePoint? =
      point?.copy(x = positionInNode.x, y = positionInNode.y)

    override fun scheduleTapDispatch(dispatchAtMillis: Long) {
      scheduledTapDispatchAtMillis = dispatchAtMillis
    }

    override fun cancelTapDispatch() {
      cancelTapDispatchCount += 1
      scheduledTapDispatchAtMillis = null
    }

    override fun launchInteraction(block: suspend () -> Unit) {
      scope.launch { block() }
    }

    override fun requestFocus(editor: Editor): Boolean {
      focused = true
      return true
    }

    override fun enqueuePointerCancel() {
      pointerCancelCount += 1
    }

    override fun requestCurrentCursorLine(version: Long) {
      requestedBringIntoViewVersions += version
    }
  }
}
