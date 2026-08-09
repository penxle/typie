package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.unit.IntSize
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.CalloutVariant
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StateField
import co.typie.editor.interaction.gestures.EditorConsecutiveTapMaxIntervalMillis
import co.typie.editor.runtime.EditorUiState
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorReadingModeInteractionTest {
  @Test
  fun `reading single tap does not request pointer guard for the applied selection`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertEquals(emptyList(), fixture.effects.pointerSelectionHeadVersions)
    }

  @Test
  fun `reading single tap waits through the consecutive tap window before hint`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)

      advanceTimeBy(EditorConsecutiveTapMaxIntervalMillis - 1)
      runCurrent()
      assertEquals(0, fixture.effects.hintCount)
      assertEquals(0, fixture.effects.focusRequestCount)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )

      advanceTimeBy(1)
      runCurrent()
      assertEquals(1, fixture.effects.hintCount)
      assertEquals(0, fixture.effects.editingRequestCount)
    }

  @Test
  fun `reading hint waits for the applied selection to be published`() =
    runTest(StandardTestDispatcher()) {
      val movedSelection =
        Selection(
          anchor = Position("text", 1, Affinity.Downstream),
          head = Position("text", 1, Affinity.Downstream),
        )
      val fixture = fixture(pointSelectionResult = movedSelection)
      fixture.fake.applySnapshot(fixture.editor)
      val surface = fixture.fake.attachSurfaceWithoutFrame(fixture.editor)
      fun deliverFrame(editorRevision: Long, frameKey: Long) {
        fixture.editor.deliverFrame(
          session = surface,
          bitmap = ImageBitmap(width = 100, height = 100),
          pixelSize = IntSize(width = 100, height = 100),
          editorRevision = editorRevision,
          frameKey = frameKey,
        )
      }
      advanceUntilIdle()
      deliverFrame(editorRevision = 1L, frameKey = 1L)
      advanceUntilIdle()

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertEquals(movedSelection, fixture.editor.appliedState.selection)
      assertEquals(FakeFfiEditor.EmptySelection, fixture.editor.publishedState.selection)
      assertEquals(0, fixture.effects.hintCount)

      advanceTimeBy(EditorConsecutiveTapMaxIntervalMillis)
      runCurrent()

      assertEquals(0, fixture.effects.hintCount)
      assertEquals(2L, fixture.editor.appliedRevision)
      assertEquals(1L, fixture.editor.publishedRevision)

      deliverFrame(editorRevision = 2L, frameKey = 2L)
      advanceUntilIdle()
      fixture.controller.onEditorStateChanged(fixture.editor.publishedState)

      assertEquals(1, fixture.effects.hintCount)
    }

  @Test
  fun `reading hint waits for a newer applied revision when the selection is unchanged`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.applySnapshot(fixture.editor)
      val surface = fixture.fake.attachSurfaceWithoutFrame(fixture.editor)
      fun deliverFrame(editorRevision: Long) {
        fixture.editor.deliverFrame(
          session = surface,
          bitmap = ImageBitmap(width = 100, height = 100),
          pixelSize = IntSize(width = 100, height = 100),
          editorRevision = editorRevision,
          frameKey = editorRevision,
        )
      }
      advanceUntilIdle()
      deliverFrame(editorRevision = 1L)
      advanceUntilIdle()

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertEquals(FakeFfiEditor.EmptySelection, fixture.editor.appliedState.selection)
      assertEquals(FakeFfiEditor.EmptySelection, fixture.editor.publishedState.selection)

      advanceTimeBy(EditorConsecutiveTapMaxIntervalMillis)
      runCurrent()
      fixture.controller.onEditorStateChanged(fixture.editor.publishedState)

      assertEquals(2L, fixture.editor.appliedRevision)
      assertEquals(1L, fixture.editor.publishedRevision)
      assertEquals(0, fixture.effects.hintCount)

      deliverFrame(editorRevision = 2L)
      advanceUntilIdle()
      fixture.controller.onEditorStateChanged(fixture.editor.publishedState)

      assertEquals(1, fixture.effects.hintCount)
    }

  @Test
  fun `editable 250 millisecond tap timer cannot present the reading hint`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture(editing = true)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.onTapTimer(nowMillis = 250L)
      advanceUntilIdle()

      assertEquals(0, fixture.effects.hintCount)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )
    }

  @Test
  fun `reading double tap applies reading selection then one editable cursor move`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceTimeBy(100L)
      fixture.controller.down(pointerId = 2L, nowMillis = 140L)
      fixture.controller.up(pointerId = 2L, nowMillis = 180L)
      advanceUntilIdle()

      assertTrue(fixture.editing)
      assertEquals(1, fixture.effects.editingRequestCount)
      assertEquals(1, fixture.effects.focusRequestCount)
      assertEquals(1, fixture.effects.softwareKeyboardRequestCount)
      assertEquals(0, fixture.effects.hintCount)
      assertEquals(
        listOf<Message>(
          Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
          Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
        ),
        fixture.fake.enqueued,
      )
      assertTrue(
        fixture.fake.enqueued.none { message ->
          (message as? Message.Selection)?.op is SelectionOp.SelectUnitAt
        }
      )
    }

  @Test
  fun `read only single tap selects without editing regardless of the edit preference`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture(readOnly = true, doubleTapToEditEnabled = false)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertFalse(fixture.editing)
      assertEquals(0, fixture.effects.editingRequestCount)
      assertEquals(0, fixture.effects.focusRequestCount)
      assertEquals(0, fixture.effects.softwareKeyboardRequestCount)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )

      advanceTimeBy(EditorConsecutiveTapMaxIntervalMillis)
      runCurrent()
      assertEquals(1, fixture.effects.hintCount)
    }

  @Test
  fun `read only tap stays selection only while editing cleanup is pending`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture(editing = true, readOnly = true)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertTrue(fixture.editing)
      assertEquals(0, fixture.effects.editingRequestCount)
      assertEquals(0, fixture.effects.focusRequestCount)
      assertEquals(0, fixture.effects.softwareKeyboardRequestCount)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )
    }

  @Test
  fun `read only double tap selects a word and starts selection drag without editing`() =
    runTest(StandardTestDispatcher()) {
      val selectedWord =
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 5, Affinity.Downstream),
        )
      val fixture = fixture(readOnly = true, unitSelectionResult = selectedWord)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      fixture.controller.down(pointerId = 2L, nowMillis = 140L)
      runCurrent()

      assertFalse(fixture.editing)
      assertEquals(0, fixture.effects.editingRequestCount)
      assertEquals(0, fixture.effects.focusRequestCount)
      assertEquals(0, fixture.effects.softwareKeyboardRequestCount)
      assertTrue(
        fixture.fake.enqueued.any { message ->
          ((message as? Message.Selection)?.op as? SelectionOp.SelectUnitAt)?.unit ==
            SelectionPointUnit.Word
        }
      )

      fixture.controller.move(pointerId = 2L, position = Offset(20f, 20f), nowMillis = 160L)
      runCurrent()

      assertEquals(EditorInteractionMode.DoubleTapSelecting, fixture.controller.interactionMode)
      assertTrue(
        fixture.fake.enqueued.any { message ->
          (message as? Message.Selection)?.op is SelectionOp.ExtendTo
        }
      )

      fixture.controller.up(pointerId = 2L, nowMillis = 180L, position = Offset(20f, 20f))
      advanceUntilIdle()
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)
      assertEquals(0, fixture.effects.hintCount)
    }

  @Test
  fun `reading Shift tap extends selection before editing activation places a collapsed cursor`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(
        pointerId = 1L,
        nowMillis = 0L,
        inputModifiers = InputModifiers(shift = true),
      )
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceTimeBy(100L)
      fixture.controller.down(
        pointerId = 2L,
        nowMillis = 140L,
        inputModifiers = InputModifiers(shift = true),
      )
      fixture.controller.up(pointerId = 2L, nowMillis = 180L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(
          Message.Selection(
            SelectionOp.ExtendTo(
              anchor = FakeFfiEditor.EmptySelection.anchor,
              headPage = 0,
              headX = 10f,
              headY = 20f,
              baseSelection = null,
              allowCollapse = true,
            )
          ),
          Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
        ),
        fixture.fake.enqueued,
      )
    }

  @Test
  fun `single tap opt out promotes and preserves the first tap for editable double tap`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture(doubleTapToEditEnabled = false)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertTrue(fixture.editing)
      assertEquals(
        Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f)),
        fixture.fake.enqueued.single(),
      )

      fixture.controller.down(pointerId = 2L, nowMillis = 120L)
      fixture.controller.up(pointerId = 2L, nowMillis = 160L)
      advanceUntilIdle()

      assertTrue(
        fixture.fake.enqueued.any { message ->
          (message as? Message.Selection)?.op is SelectionOp.SelectUnitAt
        }
      )
      assertEquals(1, fixture.effects.editingRequestCount)
      assertEquals(0, fixture.effects.hintCount)
    }

  @Test
  fun `single tap opt out collapses an existing iOS reading selection before editing`() =
    runTest(StandardTestDispatcher()) {
      val fixture =
        fixture(
          doubleTapToEditEnabled = false,
          expandedSelection = true,
          tapHitsSelection = true,
          platform = Platform.iOS,
        )
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(fixture.editing)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )
      assertEquals(1, fixture.effects.focusRequestCount)
      assertEquals(1, fixture.effects.softwareKeyboardRequestCount)
    }

  @Test
  fun `movement and pointer cancellation discard a pending reading result`() =
    runTest(StandardTestDispatcher()) {
      val moved = fixture()
      moved.fake.publishSnapshot(moved.editor)
      moved.controller.down(pointerId = 1L, nowMillis = 0L)
      moved.controller.move(pointerId = 1L, position = Offset(19f, 20f), nowMillis = 20L)
      moved.controller.up(pointerId = 1L, position = Offset(19f, 20f), nowMillis = 40L)
      advanceUntilIdle()
      assertEquals(0, moved.effects.hintCount)

      val cancelled = fixture()
      cancelled.fake.publishSnapshot(cancelled.editor)
      cancelled.controller.down(pointerId = 2L, nowMillis = 100L)
      cancelled.controller.up(pointerId = 2L, nowMillis = 140L)
      cancelled.controller.cancel()
      advanceUntilIdle()
      assertEquals(0, cancelled.effects.hintCount)
    }

  @Test
  fun `header down cancels pending reading presentation without undoing the selection`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      fixture.controller.onPointerDown(
        change =
          pointerChange(pointerId = 2L, nowMillis = 100L, pressed = true, previousPressed = false),
        position = null,
        positionInRoot = Offset(10f, 20f),
      )
      advanceUntilIdle()

      assertEquals(0, fixture.effects.hintCount)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )
    }

  @Test
  fun `reading fold control performs view action without entering editing or showing hint`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture(interactiveHit = InteractiveHit.FoldTitle(id = "fold", textRect = null))
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(Message.View(co.typie.editor.ffi.ViewOp.ToggleFold("fold"))),
        fixture.fake.enqueued,
      )
      assertFalse(fixture.editing)
      assertEquals(0, fixture.effects.hintCount)
    }

  @Test
  fun `reading callout control blocks node mutation without entering editing or showing hint`() =
    runTest(StandardTestDispatcher()) {
      val fixture =
        fixture(
          interactiveHit =
            InteractiveHit.CalloutIcon(id = "callout", nextVariant = CalloutVariant.Warning)
        )
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceUntilIdle()

      assertTrue(fixture.fake.enqueued.isEmpty())
      assertFalse(fixture.editing)
      assertEquals(0, fixture.effects.hintCount)
    }

  @Test
  fun `dismissing an existing selection menu still applies the reading tap and shows the hint`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture()
      fixture.fake.publishSnapshot(fixture.editor)
      fixture.effects.uiState.contextMenu.show(fixture.editor.publishedState)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceUntilIdle()

      assertFalse(fixture.effects.uiState.contextMenu.visible)
      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )
      assertEquals(1, fixture.effects.hintCount)
      assertFalse(fixture.editing)
    }

  @Test
  fun `reading tap replaces an existing range and still shows the edit hint`() =
    runTest(StandardTestDispatcher()) {
      val fixture = fixture(expandedSelection = true)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      advanceUntilIdle()

      assertEquals(
        listOf<Message>(Message.Selection(SelectionOp.SetAt(page = 0, x = 10f, y = 20f))),
        fixture.fake.enqueued,
      )
      assertEquals(1, fixture.effects.hintCount)
      assertFalse(fixture.effects.uiState.contextMenu.visible)
      assertFalse(fixture.editing)
    }

  @Test
  fun `reading tap that selects a node shows the hint and copy menu after confirmation`() =
    runTest(StandardTestDispatcher()) {
      val nodeSelection =
        Selection(
          anchor = Position("node", 0, Affinity.Downstream),
          head = Position("node", 1, Affinity.Downstream),
        )
      val fixture = fixture(pointSelectionResult = nodeSelection)
      fixture.fake.publishSnapshot(fixture.editor)

      fixture.controller.down(pointerId = 1L, nowMillis = 0L)
      fixture.controller.up(pointerId = 1L, nowMillis = 40L)
      runCurrent()

      assertEquals(0, fixture.effects.hintCount)
      assertFalse(fixture.effects.uiState.contextMenu.visible)
      assertEquals(0, fixture.effects.focusRequestCount)
      assertEquals(0, fixture.effects.softwareKeyboardRequestCount)

      advanceTimeBy(EditorConsecutiveTapMaxIntervalMillis)
      runCurrent()

      assertEquals(1, fixture.effects.hintCount)
      assertTrue(fixture.effects.uiState.contextMenu.isVisibleFor(fixture.editor.publishedState))
      assertFalse(fixture.editing)
    }

  private fun TestScope.fixture(
    editing: Boolean = false,
    readOnly: Boolean = false,
    doubleTapToEditEnabled: Boolean = true,
    interactiveHit: InteractiveHit? = null,
    expandedSelection: Boolean = false,
    tapHitsSelection: Boolean = false,
    pointSelectionResult: Selection? = null,
    unitSelectionResult: Selection? = null,
    platform: Platform = Platform.Desktop,
  ): Fixture {
    var currentSelection =
      if (expandedSelection) {
        Selection(
          anchor = Position("text", 0, Affinity.Downstream),
          head = Position("text", 4, Affinity.Downstream),
        )
      } else {
        FakeFfiEditor.EmptySelection
      }
    lateinit var fake: FakeFfiEditor
    fake =
      FakeFfiEditor(
        selectionProvider = { currentSelection },
        pageSizesProvider = { listOf(Size(width = 100f, height = 100f)) },
        onTick = {
          val latestSelection = fake.enqueued.lastOrNull() as? Message.Selection
          val selectionChanged =
            when (latestSelection?.op) {
              is SelectionOp.SetAt -> {
                currentSelection = pointSelectionResult ?: FakeFfiEditor.EmptySelection
                true
              }
              is SelectionOp.SelectUnitAt -> {
                if (unitSelectionResult != null) {
                  currentSelection = unitSelectionResult
                  true
                } else {
                  false
                }
              }
              else -> false
            }
          if (selectionChanged) {
            listOf(
              EditorEvent.StateChanged(listOf(StateField.Selection)),
              EditorEvent.RenderInvalidated,
            )
          } else {
            emptyList()
          }
        },
        interactiveRegionsProvider = {
          interactiveHit?.let { listOf(FakeFfiEditor.coveringRegion(it)) } ?: emptyList()
        },
        selectionHitRectsProvider = {
          if (tapHitsSelection) {
            listOf(PageRect(pageIdx = 0, rect = Rect(x = 0f, y = 0f, width = 100f, height = 100f)))
          } else {
            emptyList()
          }
        },
      )
    val editor = Editor(fake, this, StandardTestDispatcher(testScheduler))
    lateinit var fixture: Fixture
    val effects =
      TestEffects(
        scope = this,
        onRequestEditing = { requestedEditor ->
          if (requestedEditor !== editor) {
            false
          } else {
            fixture.editing = true
            true
          }
        },
      )
    val controller =
      EditorInteractionController(
        editorProvider = { editor },
        effects = effects,
        geometry = effects,
        uiStateProvider = { effects.uiState },
        platformProvider = { platform },
        readOnlyProvider = { readOnly },
        editingProvider = { fixture.editing },
        doubleTapToEditEnabledProvider = { doubleTapToEditEnabled },
      )
    controller.updateTapSlop(8f)
    fixture =
      Fixture(
        fake = fake,
        editor = editor,
        effects = effects,
        controller = controller,
        editing = editing,
      )
    return fixture
  }

  private class Fixture(
    val fake: FakeFfiEditor,
    val editor: Editor,
    val effects: TestEffects,
    val controller: EditorInteractionController,
    var editing: Boolean,
  )

  private class TestEffects(
    private val scope: TestScope,
    private val onRequestEditing: (Editor) -> Boolean,
  ) : EditorInteractionEffects, EditorInteractionGeometry {
    override val density: Float = 1f
    val uiState = EditorUiState()
    var hintCount = 0
    var editingRequestCount = 0
    var focusRequestCount = 0
    var softwareKeyboardRequestCount = 0
    val pointerSelectionHeadVersions = mutableListOf<Long>()
    private var tapSequenceConfirmationJob: Job? = null

    override fun containsDocumentInteraction(positionInRoot: Offset): Boolean = true

    override fun resolveInteractionPosition(positionInRoot: Offset): Offset = positionInRoot

    override fun isTapEligible(positionInRoot: Offset): Boolean = true

    override fun resolvePoint(positionInNode: Offset): PagePoint =
      PagePoint(page = 0, x = positionInNode.x, y = positionInNode.y)

    override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset = Offset(x, y)

    override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? = null

    override fun dispatchEdgeAutoScroll(delta: Offset): Offset = Offset.Zero

    override fun scheduleTapDispatch(dispatchAtMillis: Long) = Unit

    override fun cancelTapDispatch() = Unit

    override fun scheduleTapSequenceConfirmation(onConfirmed: () -> Unit) {
      tapSequenceConfirmationJob?.cancel()
      tapSequenceConfirmationJob = scope.launch {
        delay(EditorConsecutiveTapMaxIntervalMillis)
        onConfirmed()
      }
    }

    override fun cancelTapSequenceConfirmation() {
      tapSequenceConfirmationJob?.cancel()
      tapSequenceConfirmationJob = null
    }

    override fun scheduleLongPressDispatch(
      pointerId: Long,
      position: Offset,
      dispatchAtMillis: Long,
    ) = Unit

    override fun cancelLongPressDispatch() = Unit

    override fun launchInteraction(block: suspend () -> Unit) {
      scope.launch { block() }
    }

    override fun requestEditing(editor: Editor): Boolean {
      editingRequestCount += 1
      return onRequestEditing(editor)
    }

    override fun showReadingTapHint() {
      hintCount += 1
    }

    override fun requestFocus(editor: Editor): Boolean {
      focusRequestCount += 1
      uiState.updateFocus(true)
      return true
    }

    override fun requestSoftwareKeyboard() {
      softwareKeyboardRequestCount += 1
    }

    override fun setScrollGestureLocked(locked: Boolean) = Unit

    override fun performSelectionHaptic() = Unit

    override fun requestPointerSelectionHead(version: Long) {
      pointerSelectionHeadVersions += version
    }
  }
}

private fun EditorInteractionController.down(
  pointerId: Long,
  nowMillis: Long,
  inputModifiers: InputModifiers = InputModifiers(),
) {
  onPointerDown(
    change = pointerChange(pointerId, nowMillis, pressed = true, previousPressed = false),
    position = Offset(10f, 20f),
    inputModifiers = inputModifiers,
  )
}

private fun EditorInteractionController.move(pointerId: Long, position: Offset, nowMillis: Long) {
  onPointerMove(
    change = pointerChange(pointerId, nowMillis, pressed = true, previousPressed = true),
    position = position,
  )
}

private fun EditorInteractionController.up(
  pointerId: Long,
  nowMillis: Long,
  position: Offset = Offset(10f, 20f),
) {
  onPointerUp(
    change = pointerChange(pointerId, nowMillis, pressed = false, previousPressed = true),
    position = position,
  )
}

private fun pointerChange(
  pointerId: Long,
  nowMillis: Long,
  pressed: Boolean,
  previousPressed: Boolean,
): PointerInputChange =
  PointerInputChange(
    id = PointerId(pointerId),
    uptimeMillis = nowMillis,
    position = Offset.Zero,
    pressed = pressed,
    previousUptimeMillis = nowMillis,
    previousPosition = Offset.Zero,
    previousPressed = previousPressed,
    isInitiallyConsumed = false,
  )
