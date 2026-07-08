package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import androidx.compose.ui.geometry.Size as ComposeSize
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.CalloutVariant
import co.typie.editor.ffi.CursorMetrics
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
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.ffi.ViewOp
import co.typie.editor.interaction.gestures.EditorSelectionHandleType
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
      assertFalse(controller.hasActivePointer)
      assertEquals(1, host.cancelTapDispatchCount)
      assertEquals(1, host.pointerCancelCount)
      assertNull(controller.magnifierPosition)
    }

  @Test
  fun `tap rejected by page admission does not advance consecutive tap history`() =
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
  fun `android single tap that creates range selection opens context menu after commit`() =
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
  fun `second pinch pointer clears pending double tap drag from outside editor pointer path`() =
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
        controller.onPointerDown(
          pointerId = 3L,
          position = start + Offset(100f, 0f),
          nowMillis = 130L,
          tapEnabled = false,
        )
      )
      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)
      assertTrue(
        controller.onPointerUp(
          pointerId = 3L,
          position = start + Offset(100f, 0f),
          nowMillis = 135L,
        )
      )

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
  fun `third pinch pointer cancels active viewport zoom`() =
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

      assertFalse(
        controller.onPointerDown(pointerId = 1L, position = Offset(10f, 20f), nowMillis = 0L)
      )
      assertTrue(
        controller.onPointerDown(
          pointerId = 2L,
          position = Offset(110f, 20f),
          nowMillis = 10L,
          tapEnabled = false,
        )
      )
      assertEquals(EditorInteractionMode.ViewportZooming, controller.interactionMode)

      assertTrue(
        controller.onPointerDown(
          pointerId = 3L,
          position = Offset(210f, 20f),
          nowMillis = 20L,
          tapEnabled = false,
        )
      )

      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(
        controller.onPointerMove(pointerId = 1L, position = Offset(20f, 20f), nowMillis = 30L)
      )
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.From, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.From, down))
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)
      assertFalse(host.uiState.contextMenu.isVisibleFor(editor.state))

      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.From, Offset(22f, 50f))
      )

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(selection.head, extend.anchor)
      assertEquals(0, extend.headPage)
      assertEquals(20f, extend.headX)
      assertEquals(44f, extend.headY)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)
      assertEquals(Offset(20f, 44f), controller.magnifierPosition)

      assertTrue(controller.handleSelectionHandleDragEnd(EditorSelectionHandleType.From))
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))

      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(52f, 50f))
      )

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(50f, extend.headX)
      assertEquals(44f, extend.headY)
      assertNull(extend.baseSelection)
      assertFalse(extend.allowCollapse)

      assertTrue(controller.handleSelectionHandleDragEnd(EditorSelectionHandleType.To))
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))

      host.point = null

      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(200f, -40f))
      )
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))
      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(52f, 50f))
      )
      assertEquals(EditorInteractionMode.SelectionHandleDragging, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)
      assertEquals(Offset(50f, 44f), controller.magnifierPosition)

      controller.handleSelectionHandleDragCancel()

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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))
      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(52f, 50f))
      )
      assertTrue(controller.handleSelectionHandleDragEnd(EditorSelectionHandleType.To))
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))
      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(52f, 95f))
      )
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

      assertTrue(controller.handleSelectionHandleDragEnd(EditorSelectionHandleType.To))
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.From, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.From, down))
      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.From, Offset(16f, 30f))
      )

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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))
      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(52f, 95f))
      )

      controller.handleSelectionHandleDragCancel()
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

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertTrue(controller.handleSelectionHandleDragStart(EditorSelectionHandleType.To, down))
      assertTrue(
        controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, Offset(52f, 95f))
      )
      fake.enqueued.clear()

      advanceTimeBy(16)
      runCurrent()

      val extend = (fake.enqueued.single() as Message.Selection).op as SelectionOp.ExtendTo
      assertEquals(100f, extend.headY)
      assertFalse(extend.allowCollapse)

      assertTrue(controller.handleSelectionHandleDragEnd(EditorSelectionHandleType.To))
    }

  @Test
  fun `selection handle down only owns pending drag until movement starts drag`() =
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
      val down = Offset(42f, 30f)

      assertTrue(controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, down))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      assertFalse(controller.handleSelectionHandleDragUpdate(EditorSelectionHandleType.To, down))
      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
      assertNull(controller.magnifierPosition)

      controller.handleSelectionHandleDragCancel()

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

      assertFalse(
        controller.handleSelectionHandleDragDown(EditorSelectionHandleType.To, Offset(42f, 30f))
      )
      assertEquals(EditorInteractionMode.LongPressSelecting, controller.interactionMode)
      assertTrue(host.scrollGestureLockActive)

      assertTrue(controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 600L))
      assertEquals(EditorInteractionMode.Idle, controller.interactionMode)
      assertFalse(host.scrollGestureLockActive)
    }

  private fun EditorInteractionController.handleSelectionHandleDragDown(
    type: EditorSelectionHandleType,
    position: Offset,
  ): Boolean = selectionHandleGesture.handleDragDown(type = type, position = position)

  private fun EditorInteractionController.handleSelectionHandleDragStart(
    type: EditorSelectionHandleType,
    position: Offset,
  ): Boolean = selectionHandleGesture.handleDragStart(type = type, position = position)

  private fun EditorInteractionController.handleSelectionHandleDragUpdate(
    type: EditorSelectionHandleType,
    position: Offset,
  ): Boolean = selectionHandleGesture.handleDragUpdate(type = type, position = position)

  private fun EditorInteractionController.handleSelectionHandleDragEnd(
    type: EditorSelectionHandleType
  ): Boolean = selectionHandleGesture.handleDragEnd(type = type)

  private fun EditorInteractionController.handleSelectionHandleDragCancel() {
    selectionHandleGesture.cancel()
  }

  @Test
  fun `pending double tap drag locks scroll gesture until pointer up`() =
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
      val start = Offset(10f, 20f)

      controller.onPointerDown(pointerId = 1L, position = start, nowMillis = 0L)
      controller.onPointerUp(pointerId = 1L, position = start, nowMillis = 40L)
      advanceUntilIdle()

      controller.onPointerDown(pointerId = 2L, position = start, nowMillis = 120L)
      advanceUntilIdle()

      assertTrue(host.scrollGestureLockActive)

      controller.onPointerMove(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 140L)

      assertTrue(host.scrollGestureLockActive)

      controller.onPointerUp(pointerId = 2L, position = start + Offset(8f, 0f), nowMillis = 160L)
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

  private class TestHost(private val scope: TestScope) :
    EditorInteractionEffects, EditorInteractionGeometry {
    override var density: Float = 1f
    var scheduledTapDispatchAtMillis: Long? = null
    var scheduledLongPressDispatchAtMillis: Long? = null
    var cancelTapDispatchCount = 0
    var pointerCancelCount = 0
    var launchInteractionCount = 0
    var focused = false
    val uiState = EditorUiState()
    var scrollGestureLockActive = false
    var point: PagePoint? = PagePoint(page = 0, x = 10f, y = 20f)
    var edgeAutoScrollViewport: EditorEdgeAutoScrollViewport? = null
    var edgeAutoScrollConsumedDelta = Offset.Zero
    val requestedBringIntoViewVersions = mutableListOf<Long>()

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
      return edgeAutoScrollConsumedDelta
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
        rows =
          listOf(
            TableOverlayRow(index = 0, height = 40f, position = 40f, backgroundColor = null),
            TableOverlayRow(index = 1, height = 40f, position = 80f, backgroundColor = null),
          ),
        columns =
          listOf(
            TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f, backgroundColor = null),
            TableOverlayColumn(index = 1, widthAsPx = 50f, position = 100f, backgroundColor = null),
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
