package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import androidx.compose.ui.geometry.Size as ComposeSize
import androidx.compose.ui.unit.Velocity
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.CalloutVariant
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOp
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.ffi.ViewOp
import co.typie.editor.interaction.gestures.EditorPanGestureDriver
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorInteractionControllerTest {
  @Test
  fun `pinch sample uses the complete pair of physical root positions`() {
    val sample = resolveEditorPinchSample(listOf(Offset(100f, 200f), Offset(500f, 500f)))

    assertEquals(EditorPinchSample(focalInRootPx = Offset(300f, 350f), distancePx = 500f), sample)
    assertNull(resolveEditorPinchSample(listOf(Offset.Zero)))
    assertNull(resolveEditorPinchSample(listOf(Offset.Zero, Offset.Zero, Offset.Zero)))
  }

  @Test
  fun `pinch start cancels active pending tap stream`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)

      controller.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 0L)

      controller.applyModeEvent(EditorInteractionEvent.ViewportZoomStart)

      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)
      assertEquals(1, host.cancelTapDispatchCount)
      assertEquals(1, host.pointerCancelCount)
      assertNull(controller.magnifierPosition)
    }

  @Test
  fun `controller consumes one complete physical pinch sample at a time`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          semantics = viewportZoomEnabledSemantics(effects = host),
        )
      controller.updateTapSlop(8f)
      controller.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 0L)

      assertTrue(
        controller.onPinchSample(
          EditorPinchSample(focalInRootPx = Offset(60f, 20f), distancePx = 100f)
        )
      )
      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)
      assertEquals(1, host.pointerCancelCount)

      assertTrue(
        controller.onPinchSample(
          EditorPinchSample(focalInRootPx = Offset(70f, 25f), distancePx = 120f)
        )
      )
      assertTrue(
        controller.onPinchSample(
          EditorPinchSample(focalInRootPx = Offset(70f, 25f), distancePx = 0f)
        )
      )
      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)
      controller.onPinchEnd()

      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `tap rejected by page eligibility does not advance consecutive tap history`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L, tapEnabled = false)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)

      assertFalse(controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L))
      assertEquals(250L + 120L, host.scheduledTapDispatchAtMillis)
    }

  @Test
  fun `indirect zoom breaks consecutive tap history`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          semantics = viewportZoomEnabledSemantics(effects = host),
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      controller.onTapTimer(nowMillis = 250L)
      advanceUntilIdle()

      assertTrue(controller.beginIndirectZoom())
      controller.endIndirectZoom()

      assertFalse(controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 320L))
      controller.onPointerUp(pointerId = 2L, position = start, nowMillis = 360L)
      controller.onTapTimer(nowMillis = 570L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(
          Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
          Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
        ),
        fake.enqueued,
      )
    }

  @Test
  fun `plain drag past tap slop cancels tap timer without taking over selection`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
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
  fun `tap timer selection hit guard does not dispatch primary click`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor(selectionHitProvider = { _, _, _ -> true })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onTapTimer(nowMillis = 250L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 300L)
      advanceUntilIdle()

      assertEquals(emptyList(), fake.enqueued)
      assertEquals(emptyList(), host.requestedBringIntoViewVersions)
    }

  @Test
  fun `tap timer outside page does not open context menu for range selection`() =
    runTest(StandardTestDispatcher()) {
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { rangeSelection },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.point = PagePoint(page = -1, x = 10f, y = 20f)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onTapTimer(nowMillis = 250L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 300L)
      advanceUntilIdle()

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun `single tap on range selection hit toggles context menu without moving cursor`() =
    runTest(StandardTestDispatcher()) {
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { rangeSelection },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertTrue(host.focused)
      assertEquals(emptyList(), fake.enqueued)

      host.focused = false
      host.uiState.updateFocus(false)
      host.uiState.contextMenu.hide()

      controller.onPointerDown(pointerId = 3L, position = start, nowMillis = 1200L)
      controller.onPointerUp(pointerId = 3L, position = start, nowMillis = 1240L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertTrue(host.focused)
      assertEquals(emptyList(), fake.enqueued)

      controller.onPointerDown(pointerId = 4L, position = start, nowMillis = 1700L)
      controller.onPointerUp(pointerId = 4L, position = start, nowMillis = 1740L)
      advanceUntilIdle()

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun `android single tap on range selection hit moves cursor instead of toggling context menu`() =
    runTest(StandardTestDispatcher()) {
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { rangeSelection },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertTrue(host.focused)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fake.enqueued,
      )
    }

  @Test
  fun `single tap while editor already focused requests software keyboard`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.focused = true
      host.uiState.updateFocus(true)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(1, host.softwareKeyboardRequestCount)
    }

  @Test
  fun `single tap that grants focus does not request software keyboard`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(host.focused)
      assertEquals(0, host.softwareKeyboardRequestCount)
    }

  @Test
  fun `shift single tap dispatch extends from current selection anchor`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 1, Affinity.Downstream),
          head = Position("text", 3, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(cursorProvider = { cursorAt(x = 10f) }, selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(
        pointerId = 1L,
        position = start,
        nowMillis = 0L,
        inputModifiers = InputModifiers(shift = true),
      )
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(
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
        ),
        fake.enqueued,
      )
    }

  @Test
  fun `android single tap that creates range selection opens context menu and requests bring into view after commit`() =
    runTest(StandardTestDispatcher()) {
      val collapsedSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 0, Affinity.Downstream),
        )
      val nodeSelection =
        Selection(
          anchor = Position("node", 0, Affinity.Downstream),
          head = Position("node", 1, Affinity.Downstream),
        )
      var currentSelection = collapsedSelection
      var commitNodeSelection = false
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 10f) },
          selectionProvider = { currentSelection },
          onTick = {
            if (commitNodeSelection) {
              currentSelection = nodeSelection
            }
            emptyList()
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      commitNodeSelection = true
      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertTrue(host.focused)
      assertEquals(listOf(2L), host.requestedBringIntoViewVersions)
    }

  @Test
  fun `ios single tap that creates range selection opens context menu after commit`() =
    runTest(StandardTestDispatcher()) {
      val collapsedSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 0, Affinity.Downstream),
        )
      val nodeSelection =
        Selection(
          anchor = Position("node", 0, Affinity.Downstream),
          head = Position("node", 1, Affinity.Downstream),
        )
      var currentSelection = collapsedSelection
      var commitNodeSelection = false
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 10f) },
          selectionProvider = { currentSelection },
          onTick = {
            if (commitNodeSelection) {
              currentSelection = nodeSelection
            }
            emptyList()
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.iOS },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      commitNodeSelection = true
      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertTrue(host.focused)
    }

  @Test
  fun `context menu hides when observed editor selection changes`() =
    runTest(StandardTestDispatcher()) {
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val collapsedSelection =
        Selection(
          anchor = Position("text", 5, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      var currentSelection = rangeSelection
      val fake =
        FakeFfiEditor(
          selectionProvider = { currentSelection },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))

      currentSelection = collapsedSelection
      editor.sync {}

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))

      host.uiState.contextMenu.onEditorStateChanged(editor.state)
      host.uiState.contextMenu.showAfterSelectionCommitIfRequested(editor.state)

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `single tap timer requests bring into view for the committed cursor version`() =
    runTest(StandardTestDispatcher()) {
      var cursor = cursorAt(x = 1f)
      val fake = FakeFfiEditor(cursorProvider = { cursor })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      cursor = cursorAt(x = 5f)
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onTapTimer(nowMillis = 250L)
      advanceUntilIdle()

      val expectedMessages: List<Message> =
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)))
      assertEquals(expectedMessages, fake.enqueued)
      assertEquals(listOf(2L), host.requestedBringIntoViewVersions)
    }

  @Test
  fun `single tap on fold title chrome toggles fold instead of moving cursor`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          interactiveHitProvider = { _, _, _ ->
            InteractiveHit.FoldTitle(
              id = "fold",
              textRect = Rect(x = 20f, y = 20f, width = 100f, height = 20f),
            )
          }
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(listOf<Message>(Message.View(ViewOp.ToggleFold(id = "fold"))), fake.enqueued)
      assertEquals(emptyList(), host.requestedBringIntoViewVersions)
    }

  @Test
  fun `single tap on fold title text still moves cursor`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          interactiveHitProvider = { _, _, _ ->
            InteractiveHit.FoldTitle(
              id = "fold",
              textRect = Rect(x = 0f, y = 0f, width = 100f, height = 40f),
            )
          }
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fake.enqueued,
      )
    }

  @Test
  fun `fold title chrome tap wins over consecutive tap history`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          interactiveHitProvider = { _, _, _ ->
            InteractiveHit.FoldTitle(
              id = "fold",
              textRect = Rect(x = 0f, y = 0f, width = 20f, height = 40f),
            )
          }
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)

      controller.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 40L)
      advanceUntilIdle()

      controller.onPointerDown(pointerId = 2L, position = Offset(25f, 20f), nowMillis = 120L)
      controller.onPointerUp(pointerId = 2L, position = Offset(25f, 20f), nowMillis = 160L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(
          Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
          Message.View(ViewOp.ToggleFold(id = "fold")),
        ),
        fake.enqueued,
      )
    }

  @Test
  fun `single tap on callout icon cycles variant instead of moving cursor`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          interactiveHitProvider = { _, _, _ ->
            InteractiveHit.CalloutIcon(id = "callout", nextVariant = CalloutVariant.Warning)
          }
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(
          Message.Node(
            NodeOp.SetAttrs(
              id = "callout",
              attrs = PlainNode.Callout(variant = CalloutVariant.Warning),
            )
          )
        ),
        fake.enqueued,
      )
      assertEquals(emptyList(), host.requestedBringIntoViewVersions)
    }

  @Test
  fun `pinch start clears pending double tap drag state`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      val baselineSelectionCount = fake.enqueued.filterIsInstance<Message.Selection>().size
      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)

      controller.applyModeEvent(EditorInteractionEvent.ViewportZoomStart)
      controller.applyModeEvent(EditorInteractionEvent.ViewportZoomEnd)

      assertFalse(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(5f, 0f),
          nowMillis = 140L,
        )
      )
      assertEquals(
        emptyList<Message.Selection>(),
        fake.enqueued.filterIsInstance<Message.Selection>().drop(baselineSelectionCount),
      )
    }

  @Test
  fun `physical pinch sample clears pending double tap drag`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          semantics = viewportZoomEnabledSemantics(effects = host),
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      val baselineSelectionCount = fake.enqueued.filterIsInstance<Message.Selection>().size
      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)

      assertTrue(
        controller.onPinchSample(
          EditorPinchSample(focalInRootPx = start + Offset(50f, 0f), distancePx = 100f)
        )
      )
      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)
      controller.onPinchEnd()

      assertEquals(1, host.pointerCancelCount)
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(5f, 0f),
          nowMillis = 140L,
        )
      )
      assertEquals(
        emptyList<Message.Selection>(),
        fake.enqueued.filterIsInstance<Message.Selection>().drop(baselineSelectionCount),
      )
    }

  @Test
  fun `pointer cancellation ends active viewport zoom`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          semantics = viewportZoomEnabledSemantics(effects = host),
        )
      controller.updateTapSlop(8f)

      assertTrue(
        controller.onPinchSample(
          EditorPinchSample(focalInRootPx = Offset(60f, 20f), distancePx = 100f)
        )
      )
      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)

      controller.cancel()

      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `consecutive tap distance scales with density`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.density = 2f
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(16f)
      val firstTap = Offset(10f, 20f)
      val secondTap = Offset(40f, 20f)

      controller.onPointerDown(pointerId = 1L, position = firstTap, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = firstTap, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(controller.onPointerDown(pointerId = 2L, position = secondTap, nowMillis = 120L))
      assertTrue(host.scrollGestureLockActive)
    }

  @Test
  fun `double tap drag threshold scales with density`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.density = 2f
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(16f)
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
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)

      assertTrue(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(9f, 0f),
          nowMillis = 160L,
        )
      )
      assertEquals(EditorInteractionMode.DoubleTapSelecting, controller.interactionMode)
    }

  @Test
  fun `double tap drag extends selection directly from controller workflow`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
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
            anchor = selection.anchor,
            headPage = 0,
            headX = 15f,
            headY = 20f,
            baseSelection = selection,
            allowCollapse = false,
          )
        ),
        fake.enqueued.last(),
      )
    }

  @Test
  fun `from selection handle drag extends selection from to endpoint anchor`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(12f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)
      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))

      assertTrue(controller.moveSelectionHandlePointer(Offset(22f, 50f)))

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(selection.head, extend.anchor)
      assertEquals(0, extend.headPage)
      assertEquals(20f, extend.headX)
      assertEquals(44f, extend.headY)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)
      assertEquals(Offset(20f, 44f), controller.magnifierPosition)

      assertTrue(controller.upSelectionHandlePointer())
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
      assertNull(controller.magnifierPosition)
    }

  @Test
  fun `to selection handle drag extends selection from from endpoint anchor`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))

      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 50f)))

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(50f, extend.headX)
      assertEquals(44f, extend.headY)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)

      assertTrue(controller.upSelectionHandlePointer())
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `selection handle drag keeps consuming when pointer temporarily resolves outside pages`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))

      host.point = null

      assertTrue(controller.moveSelectionHandlePointer(Offset(200f, -40f)))
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)
      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
    }

  @Test
  fun `selection handle cancel clears drag state scroll lock and magnifier`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 50f)))
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)
      assertEquals(Offset(50f, 44f), controller.magnifierPosition)

      controller.cancel()

      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
      assertNull(controller.magnifierPosition)
    }

  @Test
  fun `selection handle drag refreshes context menu after delayed selection commit`() =
    runTest(StandardTestDispatcher()) {
      var selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val committedSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 8, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 50f)))
      assertTrue(controller.upSelectionHandlePointer())
      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))

      selection = committedSelection
      editor.sync {}
      host.uiState.contextMenu.onEditorStateChanged(editor.state)
      host.uiState.contextMenu.showAfterSelectionCommitIfRequested(editor.state)

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `selection handle edge auto-scroll extends from opposite endpoint anchor without initial selection`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 14f)
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 95f)))
      fake.enqueued.clear()

      advanceTimeBy(16)
      runCurrent()

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(0, extend.headPage)
      assertEquals(50f, extend.headX)
      assertEquals(70f, extend.headY)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)

      assertTrue(controller.upSelectionHandlePointer())
    }

  @Test
  fun `selection edge auto-scroll tracks stationary pointer across viewport movement`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 8f)
          edgeAutoScrollMovesViewport = true
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )

      assertTrue(controller.pointerDownOnSelectionHandle(Offset(42f, 30f)))
      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 95f)))
      fake.enqueued.clear()

      advanceTimeBy(80)
      runCurrent()

      assertEquals(5, host.edgeAutoScrollDispatchCount)

      assertTrue(controller.upSelectionHandlePointer())
    }

  @Test
  fun `edge auto-scroll reports position advanced by actual consumption and skips zero`() =
    runTest(StandardTestDispatcher()) {
      val testEditor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(2f, 3f)
        }
      val semantics = EditorInteractionSemantics(effects = host)
      val context =
        object : EditorGestureContext {
          override val editor = testEditor
          override val semantics = semantics
          override val effects = host
          override val geometry = host
          override val mode = EditorInteractionMode.Idle
          override val uiState = host.uiState
          override val readOnly = false
          override val platform = Platform.Desktop

          override fun applyModeEvent(event: EditorInteractionEvent) = Unit

          override fun reduceMode(event: EditorInteractionEvent) = Unit
        }
      val reportedPositions = mutableListOf<Offset>()

      semantics.edgeAutoScroll.track(edgePosition = Offset(80f, 95f), context = context) {
        reportedPositions += it
        semantics.edgeAutoScroll.stop()
      }
      advanceTimeBy(32)
      runCurrent()

      assertEquals(listOf(Offset(82f, 98f)), reportedPositions)
      assertEquals(1, host.edgeAutoScrollDispatchCount)

      host.edgeAutoScrollConsumedDelta = Offset.Zero
      semantics.edgeAutoScroll.track(edgePosition = Offset(80f, 95f), context = context) {
        reportedPositions += it
      }
      advanceTimeBy(32)
      runCurrent()
      semantics.edgeAutoScroll.stop()

      assertEquals(listOf(Offset(82f, 98f)), reportedPositions)
      assertTrue(host.edgeAutoScrollDispatchCount > 1)
    }

  @Test
  fun `selection from handle drag anchors opposite document endpoint for reverse selection`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 5, Affinity.Downstream),
          head = Position("text", 0, Affinity.Downstream),
        )
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("text", 0, Affinity.Downstream),
          toPosition = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(12f, 24f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertTrue(controller.moveSelectionHandlePointer(Offset(16f, 30f)))

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(endpoints.toPosition, extend.anchor)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)
    }

  @Test
  fun `selection handle edge auto-scroll stops after cancel`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints = selectionEndpoints()
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 14f)
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 95f)))

      controller.cancel()
      fake.enqueued.clear()
      advanceTimeBy(16)
      runCurrent()

      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
    }

  @Test
  fun `selection handle edge auto-scroll dispatches to viewport edge when scroll reaches boundary`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints = selectionEndpoints()
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 8f)
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertTrue(controller.moveSelectionHandlePointer(Offset(52f, 95f)))
      fake.enqueued.clear()

      advanceTimeBy(16)
      runCurrent()

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(100f, extend.headY)
      assertFalse(extend.allowCollapse)

      assertTrue(controller.upSelectionHandlePointer())
    }

  @Test
  fun `selection handle down only owns pending drag until movement starts drag`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          selectionEndpointsProvider = { selectionEndpoints() },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val down = Offset(42f, 30f)

      assertTrue(controller.pointerDownOnSelectionHandle(down))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      assertTrue(controller.moveSelectionHandlePointer(down))
      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
      assertNull(controller.magnifierPosition)

      controller.cancel()

      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `editor pointer stream uses geometry density when falling back from selection handle hit target`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          selectionEndpointsProvider = { selectionEndpoints() },
          selectionHitProvider = { _, _, _ -> false },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.density = 2f
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val position = Offset(84f, 60f)
      controller.updateTapSlop(16f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = position, nowMillis = 0L))
      assertTrue(controller.onPointerUp(pointerId = 1L, position = position, nowMillis = 40L))
      advanceUntilIdle()

      val op = (fake.enqueued.single() as Message.Selection).op
      assertEquals(SelectionOp.SetAt(page = 0, x = 42f, y = 30f), op)
      assertFalse(host.scrollGestureLockActive)
      assertTrue(host.focused)
    }

  @Test
  fun `editor pointer stream keeps selection hit behavior from selection handle hit target`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          selectionEndpointsProvider = { selectionEndpoints() },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val position = Offset(42f, 30f)
      controller.updateTapSlop(8f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = position, nowMillis = 0L))
      assertTrue(controller.onPointerUp(pointerId = 1L, position = position, nowMillis = 40L))
      advanceUntilIdle()

      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
      assertFalse(host.scrollGestureLockActive)
      assertTrue(host.focused)
      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `editor pointer stream starts selection handle drag from handle hit target`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val endpoints = selectionEndpoints()
      val fake =
        FakeFfiEditor(selectionProvider = { selection }, selectionEndpointsProvider = { endpoints })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(42f, 30f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(52f, 50f), nowMillis = 20L)
      )

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(50f, extend.headX)
      assertEquals(44f, extend.headY)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertEquals(Offset(50f, 44f), controller.magnifierPosition)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(52f, 50f), nowMillis = 40L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `self fling catch takes precedence over selection handle candidate`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          selectionEndpointsProvider = { selectionEndpoints() },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val driver = TestPanGestureDriver(shouldCatchTouch = true)
      val down = Offset(42f, 30f)
      controller.updateTapSlop(8f)

      assertTrue(
        controller.onPointerDown(
          pointerId = 1L,
          position = down,
          nowMillis = 0L,
          positionInRoot = down,
          touchPanDriver = driver,
        )
      )

      assertEquals(1, driver.startCount)
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())

      assertTrue(
        controller.onPointerMove(
          pointerId = 1L,
          position = Offset(43f, 30f),
          positionInRoot = Offset(43f, 30f),
          nowMillis = 20L,
        )
      )
      assertEquals(EditorInteractionMode.Panning, controller.interactionMode)
      assertEquals(listOf(Offset(1f, 0f)), driver.updates)

      controller.cancel()
    }

  @Test
  fun `pan release preserves velocity from recent movement`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val driver = TestPanGestureDriver(shouldCatchTouch = false)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(
        pointerId = 1L,
        position = start,
        positionInRoot = start,
        nowMillis = 0L,
        touchPanDriver = driver,
      )
      controller.onPointerMove(
        pointerId = 1L,
        position = start + Offset(20f, 0f),
        positionInRoot = start + Offset(20f, 0f),
        nowMillis = 10L,
      )
      controller.onPointerMove(
        pointerId = 1L,
        position = start + Offset(40f, 0f),
        positionInRoot = start + Offset(40f, 0f),
        nowMillis = 20L,
      )
      controller.onPointerUp(
        pointerId = 1L,
        position = start + Offset(40f, 0f),
        positionInRoot = start + Offset(40f, 0f),
        nowMillis = 55L,
      )

      val velocity = driver.endVelocities.single()
      assertTrue(velocity.x > 1_500f, "Expected recent pan velocity, got $velocity")
      assertTrue(velocity.y in -0.01f..0.01f, "Expected horizontal pan velocity, got $velocity")
    }

  @Test
  fun `pan release after pointer stops has no velocity`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      val driver = TestPanGestureDriver(shouldCatchTouch = false)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(
        pointerId = 1L,
        position = start,
        positionInRoot = start,
        nowMillis = 0L,
        touchPanDriver = driver,
      )
      controller.onPointerMove(
        pointerId = 1L,
        position = start + Offset(20f, 0f),
        positionInRoot = start + Offset(20f, 0f),
        nowMillis = 10L,
      )
      controller.onPointerMove(
        pointerId = 1L,
        position = start + Offset(40f, 0f),
        positionInRoot = start + Offset(40f, 0f),
        nowMillis = 20L,
      )
      controller.onPointerUp(
        pointerId = 1L,
        position = start + Offset(40f, 0f),
        positionInRoot = start + Offset(40f, 0f),
        nowMillis = 61L,
      )

      assertEquals(Velocity.Zero, driver.endVelocities.single())
    }

  @Test
  fun `editor pointer stream does not start long press from selection handle hit target`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          selectionEndpointsProvider = { selectionEndpoints() },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(42f, 30f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))

      assertFalse(controller.onLongPressTimer(pointerId = 1L, position = down, nowMillis = 500L))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      assertTrue(controller.onPointerUp(pointerId = 1L, position = down, nowMillis = 520L))
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `editor pointer stream starts table cell handle drag from table handle hit target`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(60f, 60f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(100f, 90f), nowMillis = 20L)
      )

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, extend.anchor)
      assertEquals(0, extend.headPage)
      assertEquals(100f, extend.headX)
      assertEquals(90f, extend.headY)
      assertEquals(selection, extend.baseSelection)
      assertFalse(extend.allowCollapse)
      assertEquals(EditorInteractionMode.TableCellHandleDragging, controller.interactionMode)
      assertEquals(Offset(100f, 90f), controller.magnifierPosition)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(100f, 90f), nowMillis = 40L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `table cell handle tap dispatches a normal cell tap`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(60f, 60f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertNull(host.scheduledTapDispatchAtMillis)
      assertFalse(host.scrollGestureLockActive)
      assertTrue(controller.onPointerUp(pointerId = 1L, position = down, nowMillis = 300L))
      runCurrent()

      assertEquals(
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 60f, y = 60f))),
        fake.enqueued.filterIsInstance<Message.Selection>(),
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `table cell handle delayed drag does not dispatch a pending tap first`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(60f, 60f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertNull(host.scheduledTapDispatchAtMillis)
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(100f, 90f), nowMillis = 300L)
      )

      val messages = fake.enqueued.filterIsInstance<Message.Selection>().map { it.op }
      assertEquals(1, messages.size)
      val extend = messages.single() as SelectionOp.ExtendTo
      assertEquals(selection.anchor, extend.anchor)
      assertEquals(100f, extend.headX)
      assertEquals(90f, extend.headY)
      assertEquals(selection, extend.baseSelection)
      assertFalse(extend.allowCollapse)
    }

  @Test
  fun `table cell handle drag hands off to selection handle after leaving table`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(70f, 70f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(130f, 120f), nowMillis = 20L)
      )

      val extends =
        fake.enqueued.filterIsInstance<Message.Selection>().map { it.op as SelectionOp.ExtendTo }
      assertEquals(1, extends.size)
      assertTrue(extends.all { extend -> extend.baseSelection == selection })
      assertTrue(extends.all { extend -> !extend.allowCollapse })
      assertEquals(selection.anchor, extends.single().anchor)
      assertEquals(120f, extends.single().headX)
      assertEquals(110f, extends.single().headY)
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertEquals(Offset(120f, 110f), controller.magnifierPosition)

      fake.enqueued.clear()
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(131f, 121f), nowMillis = 40L)
      )
      val continuedExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(121f, continuedExtend.headX)
      assertEquals(111f, continuedExtend.headY)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(131f, 121f), nowMillis = 60L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `table cell handle drag hands back after re-entering original table`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      var tableOverlay = tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.TableOverlays))) },
          selectionProvider = { selection },
          tableOverlaysProvider = { listOf(tableOverlay) },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(70f, 70f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(130f, 120f), nowMillis = 20L)
      )
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)

      tableOverlay = tableOverlay(isFocused = false)
      editor.sync {}
      fake.enqueued.clear()

      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(90f, 90f), nowMillis = 40L)
      )
      val reenteredExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, reenteredExtend.anchor)
      assertEquals(selection, reenteredExtend.baseSelection)
      assertEquals(80f, reenteredExtend.headX)
      assertEquals(80f, reenteredExtend.headY)
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)

      tableOverlay =
        tableOverlay(
          isFocused = true,
          cellSelection =
            TableOverlayCellSelection(anchorRow = 0, anchorCol = 0, headRow = 0, headCol = 1),
        )
      editor.sync {}
      fake.enqueued.clear()

      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(91f, 91f), nowMillis = 60L)
      )
      val continuedExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(81f, continuedExtend.headX)
      assertEquals(81f, continuedExtend.headY)
      assertEquals(selection, continuedExtend.baseSelection)
      assertEquals(EditorInteractionMode.TableCellHandleDragging, controller.interactionMode)

      fake.enqueued.clear()
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(101f, 101f), nowMillis = 70L)
      )
      val tableExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(91f, tableExtend.headX)
      assertEquals(91f, tableExtend.headY)
      assertEquals(selection, tableExtend.baseSelection)
      assertEquals(EditorInteractionMode.TableCellHandleDragging, controller.interactionMode)

      fake.enqueued.clear()
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(145f, 135f), nowMillis = 75L)
      )
      val leftAgainExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(135f, leftAgainExtend.headX)
      assertEquals(125f, leftAgainExtend.headY)
      assertEquals(selection, leftAgainExtend.baseSelection)
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(145f, 135f), nowMillis = 80L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `selection handle drag hands off to table cell handle after cell selection appears`() =
    runTest(StandardTestDispatcher()) {
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("cell-text", 0, Affinity.Downstream),
          toPosition = Position("cell-text", 2, Affinity.Downstream),
        )
      var selection = Selection(anchor = endpoints.fromPosition, head = endpoints.toPosition)
      var tableOverlay = tableOverlay(isFocused = true)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.TableOverlays))) },
          selectionProvider = { selection },
          selectionEndpointsProvider = { endpoints },
          tableOverlaysProvider = { listOf(tableOverlay) },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)

      assertTrue(
        controller.onPointerDown(pointerId = 1L, position = Offset(42f, 30f), nowMillis = 0L)
      )
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(52f, 50f), nowMillis = 20L)
      )
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)

      selection = Selection(anchor = endpoints.fromPosition, head = endpoints.toPosition)
      tableOverlay =
        tableOverlay(
          isFocused = true,
          cellSelection =
            TableOverlayCellSelection(anchorRow = 0, anchorCol = 0, headRow = 0, headCol = 1),
        )
      editor.sync {}
      fake.enqueued.clear()

      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(64f, 60f), nowMillis = 40L)
      )
      val handoffExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, handoffExtend.anchor)
      assertEquals(selection, handoffExtend.baseSelection)
      assertEquals(62f, handoffExtend.headX)
      assertEquals(54f, handoffExtend.headY)
      assertEquals(EditorInteractionMode.TableCellHandleDragging, controller.interactionMode)

      fake.enqueued.clear()
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(72f, 70f), nowMillis = 60L)
      )
      val tableExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(selection, tableExtend.baseSelection)
      assertEquals(70f, tableExtend.headX)
      assertEquals(64f, tableExtend.headY)
      assertEquals(EditorInteractionMode.TableCellHandleDragging, controller.interactionMode)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(72f, 70f), nowMillis = 80L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `selection handle drag stays textual for single-cell table selection`() =
    runTest(StandardTestDispatcher()) {
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 0f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 0f, height = 8f)),
          fromPosition = Position("cell-text", 0, Affinity.Downstream),
          toPosition = Position("cell-text", 2, Affinity.Downstream),
        )
      val selection = Selection(anchor = endpoints.fromPosition, head = endpoints.toPosition)
      var tableOverlay = tableOverlay(isFocused = true)
      val fake =
        FakeFfiEditor(
          onTick = { listOf(EditorEvent.StateChanged(listOf(StateField.TableOverlays))) },
          selectionProvider = { selection },
          selectionEndpointsProvider = { endpoints },
          tableOverlaysProvider = { listOf(tableOverlay) },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)

      assertTrue(
        controller.onPointerDown(pointerId = 1L, position = Offset(42f, 30f), nowMillis = 0L)
      )
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(52f, 50f), nowMillis = 20L)
      )
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)

      tableOverlay =
        tableOverlay(
          isFocused = true,
          cellSelection =
            TableOverlayCellSelection(anchorRow = 0, anchorCol = 0, headRow = 0, headCol = 0),
        )
      editor.sync {}
      fake.enqueued.clear()

      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(62f, 60f), nowMillis = 40L)
      )
      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(60f, extend.headX)
      assertEquals(54f, extend.headY)
      assertNull(extend.baseSelection)
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(62f, 60f), nowMillis = 60L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
    }

  @Test
  fun `table cell handle edge auto-scroll dispatches with base cell selection`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 8f)
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(60f, 60f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(80f, 95f), nowMillis = 20L)
      )
      fake.enqueued.clear()

      advanceTimeBy(16)
      runCurrent()

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, extend.anchor)
      assertEquals(selection, extend.baseSelection)
      assertEquals(100f, extend.headY)
      assertFalse(extend.allowCollapse)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(80f, 95f), nowMillis = 40L)
      )
    }

  @Test
  fun `column resize drag locks scroll only while active`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateColumnResizeSlop(8f)
      val down = Offset(60f, 30f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertFalse(host.scrollGestureLockActive)

      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(70f, 30f), nowMillis = 20L)
      )
      assertTrue(host.scrollGestureLockActive)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(70f, 30f), nowMillis = 40L)
      )
      assertFalse(host.scrollGestureLockActive)

      assertTrue(controller.onPointerDown(pointerId = 2L, position = down, nowMillis = 60L))
      assertTrue(
        controller.onPointerMove(pointerId = 2L, position = Offset(70f, 30f), nowMillis = 80L)
      )
      assertTrue(host.scrollGestureLockActive)

      controller.cancel()

      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `column resize edge auto-scroll keeps handle aligned with horizontal viewport movement`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val overlay =
        tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0)
          .copy(
            bounds = Rect(x = 10f, y = 20f, width = 120f, height = 80f),
            contentWidth = 120f,
            columns =
              listOf(
                TableOverlayColumn(index = 0, widthAsPx = 60f, position = 60f),
                TableOverlayColumn(index = 1, widthAsPx = 60f, position = 120f),
              ),
          )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = { listOf(overlay) },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(2f, 8f)
          edgeAutoScrollMovesViewport = true
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateColumnResizeSlop(8f)

      assertTrue(
        controller.onPointerDown(pointerId = 1L, position = Offset(70f, 30f), nowMillis = 0L)
      )
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(80f, 95f), nowMillis = 20L)
      )

      advanceTimeBy(16)
      runCurrent()

      host.edgeAutoScrollConsumedDelta = Offset(0f, 8f)
      advanceTimeBy(16)
      runCurrent()

      assertTrue(
        controller.onPointerMove(
          pointerId = 1L,
          position = Offset(82f, 111f),
          positionInRoot = Offset(80f, 95f),
          nowMillis = 52L,
        )
      )
      assertTrue(
        controller.onPointerUp(
          pointerId = 1L,
          position = Offset(82f, 111f),
          positionInRoot = Offset(80f, 95f),
          nowMillis = 56L,
        )
      )
      assertEquals(
        listOf(
          Message.Node(NodeOp.Table(id = "table", op = TableOp.SetColumnWidths(listOf(0.6f, 0.4f))))
        ),
        fake.enqueued.filterIsInstance<Message.Node>(),
      )
    }

  @Test
  fun `table cell handle edge auto-scroll hands off after leaving table`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("cell-text", 0, Affinity.Downstream),
          head = Position("cell-text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          selectionProvider = { selection },
          tableOverlaysProvider = {
            listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 1f)
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val down = Offset(60f, 60f)

      assertTrue(controller.onPointerDown(pointerId = 1L, position = down, nowMillis = 0L))
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(80f, 95f), nowMillis = 20L)
      )
      fake.enqueued.clear()
      host.edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 120f))

      advanceTimeBy(16)
      runCurrent()

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, extend.anchor)
      assertEquals(selection, extend.baseSelection)
      assertEquals(120f, extend.headY)
      assertFalse(extend.allowCollapse)
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      fake.enqueued.clear()
      assertTrue(
        controller.onPointerMove(pointerId = 1L, position = Offset(80f, 96f), nowMillis = 56L)
      )
      val continuedExtend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(121f, continuedExtend.headY)
      assertEquals(selection, continuedExtend.baseSelection)

      assertTrue(
        controller.onPointerUp(pointerId = 1L, position = Offset(80f, 96f), nowMillis = 72L)
      )
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `selection handle drag cannot interrupt active long press interaction`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.iOS },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      assertEquals(EditorInteractionMode.LongPressSelecting, controller.interactionMode)

      assertFalse(controller.canApplyModeEvent(EditorInteractionEvent.SelectionHandleDragStart))
      assertEquals(EditorInteractionMode.LongPressSelecting, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      assertTrue(controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 600L))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  private fun EditorInteractionController.pointerDownOnSelectionHandle(position: Offset): Boolean =
    onPointerDown(pointerId = SelectionHandleTestPointerId, position = position, nowMillis = 0L)

  private fun EditorInteractionController.moveSelectionHandlePointer(position: Offset): Boolean =
    onPointerMove(pointerId = SelectionHandleTestPointerId, position = position, nowMillis = 16L)

  private fun EditorInteractionController.upSelectionHandlePointer(): Boolean =
    onPointerUp(pointerId = SelectionHandleTestPointerId, position = Offset.Zero, nowMillis = 32L)

  @Test
  fun `pending double tap drag owns pointer sequence over pan until pointer up`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val driver = TestPanGestureDriver(shouldCatchTouch = false)
      val start = Offset(10f, 20f)

      controller.onPointerDown(
        pointerId = 1L,
        position = start,
        positionInRoot = start,
        nowMillis = 0L,
        touchPanDriver = driver,
      )
      controller.onPointerUp(
        pointerId = 1L,
        position = start,
        positionInRoot = start,
        nowMillis = 40L,
      )
      advanceUntilIdle()

      controller.onPointerDown(
        pointerId = 2L,
        position = start,
        positionInRoot = start,
        nowMillis = 120L,
        touchPanDriver = driver,
      )
      advanceUntilIdle()

      assertTrue(host.scrollGestureLockActive)

      val moved = start + Offset(20f, 0f)
      controller.onPointerMove(
        pointerId = 2L,
        position = moved,
        positionInRoot = moved,
        nowMillis = 140L,
      )

      assertEquals(0, driver.startCount)
      assertEquals(EditorInteractionMode.DoubleTapSelecting, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      controller.onPointerUp(
        pointerId = 2L,
        position = moved,
        positionInRoot = moved,
        nowMillis = 160L,
      )
      advanceUntilIdle()

      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `double tap drag keeps pending extension when pointer up beats word selection commit`() =
    runTest(StandardTestDispatcher()) {
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      val baselineSelectionCount = fake.enqueued.filterIsInstance<Message.Selection>().size

      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      controller.onPointerMove(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 140L)
      controller.onPointerUp(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 150L)
      advanceUntilIdle()

      val extend =
        fake.enqueued
          .filterIsInstance<Message.Selection>()
          .drop(baselineSelectionCount)
          .map { it.op }
          .filterIsInstance<SelectionOp.ExtendTo>()
          .single()
      assertEquals(selection, extend.baseSelection)
      assertEquals(18f, extend.headX)
      assertFalse(extend.allowCollapse)
    }

  @Test
  fun `double tap drag can shrink back to the initial selected word range`() =
    runTest(StandardTestDispatcher()) {
      val baseSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val expandedSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 12, Affinity.Downstream),
        )
      var currentSelection = baseSelection
      val fake = FakeFfiEditor(selectionProvider = { currentSelection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
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
      assertEquals(baseSelection, extend.baseSelection)
      assertEquals(15f, extend.headX)
      assertFalse(extend.allowCollapse)
    }

  @Test
  fun `double tap drag edge auto-scroll keeps materialized initial selection`() =
    runTest(StandardTestDispatcher()) {
      val baseSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { baseSelection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host =
        TestHost(this).apply {
          edgeAutoScrollViewport = testEdgeAutoScrollViewport(ComposeRect(0f, 0f, 100f, 100f))
          edgeAutoScrollConsumedDelta = Offset(0f, 14f)
        }
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      advanceUntilIdle()
      controller.onPointerMove(pointerId = 2L, position = Offset(22f, 95f), nowMillis = 140L)
      fake.enqueued.clear()

      advanceTimeBy(16)
      runCurrent()

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(baseSelection, extend.baseSelection)
      assertEquals(baseSelection.anchor, extend.anchor)
      assertEquals(22f, extend.headX)
      assertEquals(70f, extend.headY)
      assertFalse(extend.allowCollapse)

      assertTrue(
        controller.onPointerUp(pointerId = 2L, position = Offset(22f, 95f), nowMillis = 180L)
      )
      advanceUntilIdle()
    }

  @Test
  fun `android long press starts word selection and extends after fresh selection materializes`() =
    runTest(StandardTestDispatcher()) {
      val wordSelection =
        Selection(
          anchor = Position("word", 0, Affinity.Downstream),
          head = Position("word", 5, Affinity.Downstream),
        )
      var currentSelection =
        Selection(
          anchor = Position("old", 0, Affinity.Downstream),
          head = Position("old", 0, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { currentSelection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      assertFalse(controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L))
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      advanceUntilIdle()

      assertEquals(EditorInteractionMode.LongPressWordSelecting, controller.interactionMode)
      assertEquals(start, controller.magnifierPosition)
      assertEquals(
        listOf<Message>(
          Message.Selection(
            SelectionOp.SelectUnitAt(page = 0, x = 10f, y = 20f, unit = SelectionPointUnit.Word)
          )
        ),
        fake.enqueued,
      )

      fake.enqueued.clear()
      currentSelection = wordSelection
      editor.sync {}

      assertTrue(
        controller.onPointerMove(
          pointerId = 1L,
          position = start + Offset(12f, -6f),
          nowMillis = 620L,
        )
      )

      val extend = (fake.enqueued.last() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(wordSelection, extend.baseSelection)
      assertEquals(22f, extend.headX)
      assertFalse(extend.allowCollapse)
      assertEquals(start + Offset(12f, -6f), controller.magnifierPosition)

      assertTrue(controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 700L))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertNull(controller.magnifierPosition)
      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `android long press ending before word selection commit opens context menu after selection settles`() =
    runTest(StandardTestDispatcher()) {
      val wordSelection =
        Selection(
          anchor = Position("word", 0, Affinity.Downstream),
          head = Position("word", 5, Affinity.Downstream),
        )
      var currentSelection =
        Selection(
          anchor = Position("old", 0, Affinity.Downstream),
          head = Position("old", 0, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { currentSelection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      assertTrue(controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 520L))

      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertNull(controller.magnifierPosition)
      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))

      currentSelection = wordSelection
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `pointer up before long press timer cancels pending long press`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)

      assertFalse(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertNull(controller.magnifierPosition)
      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun `fresh pan can start after long press ends`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.iOS },
        )
      val driver = TestPanGestureDriver(shouldCatchTouch = false)
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(
        pointerId = 1L,
        position = start,
        positionInRoot = start,
        nowMillis = 0L,
        touchPanDriver = driver,
      )
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      assertTrue(
        controller.onPointerUp(
          pointerId = 1L,
          position = start,
          positionInRoot = start,
          nowMillis = 520L,
        )
      )

      controller.onPointerDown(
        pointerId = 2L,
        position = start,
        positionInRoot = start,
        nowMillis = 600L,
        touchPanDriver = driver,
      )
      assertFalse(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(4f, 0f),
          positionInRoot = start + Offset(4f, 0f),
          nowMillis = 610L,
        )
      )
      assertTrue(
        controller.onPointerMove(
          pointerId = 2L,
          position = start + Offset(12f, 0f),
          positionInRoot = start + Offset(12f, 0f),
          nowMillis = 620L,
        )
      )

      assertEquals(EditorInteractionMode.Panning, controller.interactionMode)
      assertEquals(listOf(Offset(4f, 0f)), driver.updates)
    }

  @Test
  fun `android long press uses engine cursor hit result for cursor mode admission`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 10f) },
          cursorHitProvider = { _, _, _ -> false },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      advanceUntilIdle()

      assertEquals(EditorInteractionMode.LongPressWordSelecting, controller.interactionMode)
      assertEquals(
        listOf<Message>(
          Message.Selection(
            SelectionOp.SelectUnitAt(page = 0, x = 10f, y = 20f, unit = SelectionPointUnit.Word)
          )
        ),
        fake.enqueued,
      )
    }

  @Test
  fun `ios long press keeps cursor move mode instead of word selection`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.iOS },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      advanceUntilIdle()

      assertEquals(EditorInteractionMode.LongPressSelecting, controller.interactionMode)
      assertTrue(
        controller.onPointerMove(
          pointerId = 1L,
          position = start + Offset(12f, -6f),
          nowMillis = 620L,
        )
      )
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 22f, y = 14f))),
        fake.enqueued,
      )
    }

  @Test
  fun `cursor long press move does not queue suspend interactions per frame`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.iOS },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      repeat(3) { index ->
        assertTrue(
          controller.onPointerMove(
            pointerId = 1L,
            position = start + Offset(x = index.toFloat(), y = 0f),
            nowMillis = 520L + index,
          )
        )
      }

      assertEquals(0, host.launchInteractionCount)
      assertEquals(3, fake.enqueued.size)
    }

  @Test
  fun `long press cursor move locks desktop drag scroll until pointer up`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Desktop },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))

      assertTrue(host.scrollGestureLockActive)

      assertTrue(
        controller.onPointerMove(
          pointerId = 1L,
          position = start + Offset(12f, 0f),
          nowMillis = 520L,
        )
      )
      assertTrue(host.scrollGestureLockActive)

      controller.onPointerUp(pointerId = 1L, position = start + Offset(12f, 0f), nowMillis = 540L)

      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `long press cancel clears desktop drag scroll lock`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor()
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Desktop },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      assertTrue(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      assertTrue(host.scrollGestureLockActive)

      controller.cancel()

      assertFalse(host.scrollGestureLockActive)
    }

  @Test
  fun `android long press on range selection hit is rejected`() =
    runTest(StandardTestDispatcher()) {
      val rangeSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 10f) },
          selectionProvider = { rangeSelection },
          selectionHitProvider = { _, _, _ -> true },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
          platformProvider = { Platform.Android },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)

      assertFalse(controller.onLongPressTimer(pointerId = 1L, position = start, nowMillis = 500L))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertNull(controller.magnifierPosition)
      assertEquals(emptyList(), fake.enqueued)
    }

  @Test
  fun `same cursor single tap toggles context menu state`() =
    runTest(StandardTestDispatcher()) {
      var cursor = cursorAt(x = 10f)
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursor },
          selectionProvider = {
            Selection(
              anchor = Position("text", 0, Affinity.Downstream),
              head = Position("text", 0, Affinity.Downstream),
            )
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.uiState.updateFocus(true)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onTapTimer(nowMillis = 250L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 300L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))

      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 700L)
      controller.onTapTimer(nowMillis = 950L)
      controller.onPointerUp(pointerId = 2L, position = start, nowMillis = 1000L)
      advanceUntilIdle()

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `same cursor tap that restores focus does not open context menu`() =
    runTest(StandardTestDispatcher()) {
      val fake =
        FakeFfiEditor(
          cursorProvider = { cursorAt(x = 10f) },
          selectionProvider = {
            Selection(
              anchor = Position("text", 0, Affinity.Downstream),
              head = Position("text", 0, Affinity.Downstream),
            )
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onTapTimer(nowMillis = 250L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 300L)
      advanceUntilIdle()

      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))
      assertTrue(host.focused)
    }

  @Test
  fun `context menu stays when observed editor cursor changes without selection change`() =
    runTest(StandardTestDispatcher()) {
      var cursor = cursorAt(x = 10f)
      val collapsedSelection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 0, Affinity.Downstream),
        )
      val fake =
        FakeFfiEditor(cursorProvider = { cursor }, selectionProvider = { collapsedSelection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      host.uiState.updateFocus(true)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onTapTimer(nowMillis = 250L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 300L)
      advanceUntilIdle()

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))

      cursor = cursorAt(x = 20f)
      editor.sync {}

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))

      host.uiState.contextMenu.onEditorStateChanged(editor.state)
      host.uiState.contextMenu.showAfterSelectionCommitIfRequested(editor.state)

      assertTrue(host.uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `second pointer cancels pending double tap drag before it can extend selection`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
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
      val selection =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fake = FakeFfiEditor(selectionProvider = { selection })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
      editor.sync {}
      val host = TestHost(this)
      val controller =
        EditorInteractionController(
          editorProvider = { editor },
          effects = host,
          geometry = host,
          uiStateProvider = { host.uiState },
        )
      controller.updateTapSlop(8f)
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()
      val baselineSelectionCount = fake.enqueued.filterIsInstance<Message.Selection>().size

      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      controller.onPointerMove(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 140L)

      assertFalse(controller.onPointerDown(pointerId = 3L, position = start, nowMillis = 150L))
      advanceUntilIdle()

      assertEquals(1, host.pointerCancelCount)
      assertEquals(
        emptyList<SelectionOp.ExtendTo>(),
        fake.enqueued
          .filterIsInstance<Message.Selection>()
          .drop(baselineSelectionCount)
          .map { it.op }
          .filterIsInstance<SelectionOp.ExtendTo>(),
      )
    }

  private class TestPanGestureDriver(override val shouldCatchTouch: Boolean) :
    EditorPanGestureDriver {
    override val touchSlop: Float = 8f
    override val maximumFlingVelocity: Float = 10_000f
    var startCount = 0
    val endVelocities = mutableListOf<Velocity>()
    val updates = mutableListOf<Offset>()

    override fun start(): Boolean {
      startCount += 1
      return true
    }

    override fun markPanStarted() = Unit

    override fun update(delta: Offset) {
      updates += delta
    }

    override fun end(velocity: Velocity) {
      endVelocities += velocity
    }

    override fun cancel() = Unit
  }

  private class TestHost(private val scope: TestScope) :
    EditorInteractionEffects, EditorInteractionGeometry {
    override var density: Float = 1f
    var scheduledTapDispatchAtMillis: Long? = null
    var scheduledLongPressDispatchAtMillis: Long? = null
    var cancelTapDispatchCount = 0
    var pointerCancelCount = 0
    var launchInteractionCount = 0
    var focused = false
    var softwareKeyboardRequestCount = 0
    val uiState = EditorUiState()
    var scrollGestureLockActive = false
    var point: PagePoint? = PagePoint(page = 0, x = 10f, y = 20f)
    var edgeAutoScrollViewport: EditorEdgeAutoScrollViewport? = null
    var edgeAutoScrollConsumedDelta = Offset.Zero
    var edgeAutoScrollDispatchCount = 0
    var edgeAutoScrollMovesViewport = false
    val requestedBringIntoViewVersions = mutableListOf<Long>()

    override fun containsDocumentInteraction(positionInRoot: Offset): Boolean = true

    override fun resolveInteractionPosition(positionInRoot: Offset): Offset = positionInRoot

    override fun isTapEligible(positionInRoot: Offset): Boolean = true

    override fun resolvePoint(positionInNode: Offset): PagePoint? {
      if (density <= 0f) {
        return null
      }
      return point?.copy(x = positionInNode.x / density, y = positionInNode.y / density)
    }

    override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? {
      if (density <= 0f) {
        return null
      }
      return Offset(x = x * density, y = y * density)
    }

    override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? =
      edgeAutoScrollViewport

    override fun dispatchEdgeAutoScroll(delta: Offset): Offset {
      edgeAutoScrollDispatchCount += 1
      val consumed = edgeAutoScrollConsumedDelta
      if (edgeAutoScrollMovesViewport) {
        edgeAutoScrollViewport = edgeAutoScrollViewport?.let { viewport ->
          viewport.copy(
            rect = viewport.rect.translate(translateX = consumed.x, translateY = consumed.y)
          )
        }
      }
      return consumed
    }

    override fun scheduleTapDispatch(dispatchAtMillis: Long) {
      scheduledTapDispatchAtMillis = dispatchAtMillis
    }

    override fun cancelTapDispatch() {
      cancelTapDispatchCount += 1
      scheduledTapDispatchAtMillis = null
    }

    override fun scheduleLongPressDispatch(
      pointerId: Long,
      position: Offset,
      dispatchAtMillis: Long,
    ) {
      scheduledLongPressDispatchAtMillis = dispatchAtMillis
    }

    override fun cancelLongPressDispatch() {
      scheduledLongPressDispatchAtMillis = null
    }

    override fun launchInteraction(block: suspend () -> Unit) {
      launchInteractionCount += 1
      scope.launch { block() }
    }

    override fun requestFocus(editor: Editor): Boolean {
      focused = true
      uiState.updateFocus(true)
      return true
    }

    override fun requestSoftwareKeyboard() {
      softwareKeyboardRequestCount += 1
    }

    override fun enqueuePointerCancel() {
      pointerCancelCount += 1
    }

    override fun setScrollGestureLocked(locked: Boolean) {
      scrollGestureLockActive = locked
    }

    override fun performSelectionHaptic() = Unit

    override fun requestCurrentSelectionHead(version: Long) {
      requestedBringIntoViewVersions += version
    }
  }

  private companion object {
    const val SelectionHandleTestPointerId = 1L

    fun viewportZoomEnabledSemantics(
      effects: EditorInteractionEffects
    ): EditorInteractionSemantics {
      val layoutSpec =
        EditorDocumentLayoutSpec.Paginated(
          pageWidth = 720f,
          pageHeight = 960f,
          pageMarginTop = 0f,
          pageMarginBottom = 0f,
          pageMarginLeft = 0f,
          pageMarginRight = 0f,
        )
      val pageSizes = listOf(PageSize(width = 720f, height = 960f))
      val zoomController = EditorZoomController()
      val viewportState =
        EditorViewportState().apply {
          updateMeasuredBounds(
            viewportSize = ComposeSize(width = 100f, height = 120f),
            contentSize = ComposeSize(width = 2000f, height = 2000f),
          )
        }
      val uiState =
        EditorUiState().apply {
          updateDisplayZoom(1f)
          updatePageOffset(page = 0, offset = Offset.Zero)
          updateEditorBounds(
            boundsInRoot = ComposeRect(left = 0f, top = 0f, right = 720f, bottom = 2000f),
            density = 1f,
          )
        }
      zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = 720f)

      return EditorInteractionSemantics(effects = effects).apply {
        viewportZoom.configure(
          EditorViewportZoomSemanticConfig(
            layoutSpec = layoutSpec,
            zoomController = zoomController,
            viewportState = viewportState,
            uiState = uiState,
            pageSizes = pageSizes,
            viewportWidth = 720f,
            density = 1f,
            onZoomSnap = {},
          )
        )
      }
    }

    fun cursorAt(x: Float): CursorMetrics =
      CursorMetrics(
        pageIdx = 0,
        caret = Rect(x = x, y = 0f, width = 1f, height = 12f),
        line = Rect(x = 0f, y = 0f, width = 100f, height = 12f),
      )

    fun selectionEndpoints(): SelectionEndpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )

    fun tableOverlay(
      isFocused: Boolean = false,
      focusedRowIndex: Int? = null,
      focusedColIndex: Int? = null,
      cellSelection: TableOverlayCellSelection? = null,
    ): TableOverlay =
      TableOverlay(
        tableId = "table",
        pageIdx = 0,
        bounds = Rect(x = 10f, y = 20f, width = 100f, height = 80f),
        borderStyle = TableBorderStyle.Solid,
        align = Alignment.Left,
        proportion = 1f,
        contentWidth = 100f,
        minProportionWidth = 83f,
        maxProportionWidth = 100f,
        rows =
          listOf(
            TableOverlayRow(index = 0, height = 40f, position = 40f),
            TableOverlayRow(index = 1, height = 40f, position = 80f),
          ),
        columns =
          listOf(
            TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f),
            TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f),
          ),
        rowCount = 2,
        isLastRowFragment = true,
        isFocused = isFocused,
        focusedRowIndex = focusedRowIndex,
        focusedColIndex = focusedColIndex,
        cellSelection = cellSelection,
      )

    fun testEdgeAutoScrollViewport(rect: ComposeRect): EditorEdgeAutoScrollViewport =
      EditorEdgeAutoScrollViewport(rect = rect, density = 1f)
  }
}
