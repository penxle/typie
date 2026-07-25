package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.PointerInputChange
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.FakeFfiEditor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.Size
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.ScrollGestureLockState
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorInteractionScopeTest {
  @Test
  fun `reset cancels a pending tap sequence confirmation`() =
    runTest(StandardTestDispatcher()) {
      val scope =
        EditorInteractionScope(coroutineScope = this, platformProvider = { Platform.Desktop })
      var confirmed = false

      scope.scheduleTapSequenceConfirmation { confirmed = true }
      scope.reset()
      advanceUntilIdle()

      assertFalse(confirmed)
    }

  @Test
  fun `editing mode change cancels the latest replacement tap confirmation`() =
    runTest(StandardTestDispatcher()) {
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val scope =
        EditorInteractionScope(coroutineScope = this, platformProvider = { Platform.Desktop })
      var editing = false
      var firstConfirmed = false
      var replacementConfirmed = false

      updateScope(scope = scope, editor = editor, editing = { editing })
      scope.scheduleTapSequenceConfirmation { firstConfirmed = true }
      scope.scheduleTapSequenceConfirmation { replacementConfirmed = true }
      runCurrent()
      editing = true
      updateScope(scope = scope, editor = editor, editing = { editing })
      advanceUntilIdle()

      assertFalse(firstConfirmed)
      assertFalse(replacementConfirmed)
    }

  @Test
  fun `external editing promotion consumes the active reading pointer without redispatching it`() =
    runTest(StandardTestDispatcher()) {
      val fake = FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 400f, height = 700f)) })
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler)).also { it.sync {} }
      val uiState =
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 700f),
            density = 1f,
          )
          updateEditorBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 700f),
            density = 1f,
          )
          updatePageOffset(page = 0, offset = Offset.Zero)
        }
      val scope =
        EditorInteractionScope(coroutineScope = this, platformProvider = { Platform.Desktop })
      var editing = false
      updateScope(scope = scope, editor = editor, editing = { editing }, uiState = uiState)

      scope.controller.onPointerDown(
        change = pointerDown(id = 1L, uptimeMillis = 0L),
        position = Offset(10f, 20f),
      )

      editing = true
      updateScope(scope = scope, editor = editor, editing = { editing }, uiState = uiState)

      assertTrue(
        scope.controller.onPointerUp(
          change = pointerUp(id = 1L, uptimeMillis = 40L),
          position = Offset(10f, 20f),
        )
      )
      advanceUntilIdle()
      assertTrue(fake.enqueued.isEmpty())
    }

  @Test
  fun `accepted double tap promotion preserves its editable node menu through scope update`() =
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
      lateinit var fake: FakeFfiEditor
      fake =
        FakeFfiEditor(
          pageSizesProvider = { listOf(Size(width = 400f, height = 700f)) },
          selectionProvider = { currentSelection },
          onTick = {
            if ((fake.enqueued.lastOrNull() as? Message.Selection)?.op is SelectionOp.SetAt) {
              currentSelection = nodeSelection
            }
            emptyList()
          },
        )
      val editor = Editor(fake, this, StandardTestDispatcher(testScheduler)).also { it.sync {} }
      val uiState =
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 700f),
            density = 1f,
          )
          updateEditorBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 700f),
            density = 1f,
          )
          updatePageOffset(page = 0, offset = Offset.Zero)
        }
      val scope =
        EditorInteractionScope(coroutineScope = this, platformProvider = { Platform.Desktop })
      var editing = false
      val onRequestEditing: (Editor) -> Boolean = {
        editing = true
        true
      }
      updateScope(
        scope = scope,
        editor = editor,
        editing = { editing },
        uiState = uiState,
        onRequestEditing = onRequestEditing,
      )

      scope.controller.tap(pointerId = 1L, position = Offset(10f, 20f), downAtMillis = 0L)
      runCurrent()
      scope.controller.onPointerDown(
        change = pointerDown(id = 2L, uptimeMillis = 140L),
        position = Offset(10f, 20f),
      )
      updateScope(
        scope = scope,
        editor = editor,
        editing = { editing },
        uiState = uiState,
        onRequestEditing = onRequestEditing,
      )
      scope.controller.onPointerUp(
        change = pointerUp(id = 2L, uptimeMillis = 180L),
        position = Offset(10f, 20f),
      )
      advanceUntilIdle()

      assertTrue(editing)
      assertEquals(
        2,
        fake.enqueued.filterIsInstance<Message.Selection>().count { it.op is SelectionOp.SetAt },
      )
      assertTrue(uiState.contextMenu.isVisibleFor(editor.state))
    }

  @Test
  fun `editor replacement clears the previous editor reading tap history`() =
    runTest(StandardTestDispatcher()) {
      fun editor() =
        Editor(
            FakeFfiEditor(pageSizesProvider = { listOf(Size(width = 400f, height = 700f)) }),
            this,
            StandardTestDispatcher(testScheduler),
          )
          .also { it.sync {} }

      val firstEditor = editor()
      val secondEditor = editor()
      val uiState =
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 700f),
            density = 1f,
          )
          updateEditorBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 700f),
            density = 1f,
          )
          updatePageOffset(page = 0, offset = Offset.Zero)
        }
      val scope =
        EditorInteractionScope(coroutineScope = this, platformProvider = { Platform.Desktop })
      var editingRequestCount = 0
      val onRequestEditing: (Editor) -> Boolean = {
        editingRequestCount += 1
        true
      }

      updateScope(
        scope = scope,
        editor = firstEditor,
        editing = { false },
        uiState = uiState,
        onRequestEditing = onRequestEditing,
      )
      scope.controller.tap(pointerId = 1L, position = Offset(10f, 20f), downAtMillis = 0L)

      updateScope(
        scope = scope,
        editor = secondEditor,
        editing = { false },
        uiState = uiState,
        onRequestEditing = onRequestEditing,
      )
      scope.controller.tap(pointerId = 2L, position = Offset(10f, 20f), downAtMillis = 120L)

      assertEquals(0, editingRequestCount)
    }

  @Test
  fun `editor state observation is ignored before editor attaches`() =
    runTest(StandardTestDispatcher()) {
      val scope = EditorInteractionScope(coroutineScope = this)

      scope.update(
        editor = null,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = EditorUiState(),
        density = 1f,
        visibleArea = EditorVisibleArea(),
        viewportState = EditorViewportState(),
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        onSelectionHaptic = {},
        onRequestSoftwareKeyboard = {},
      )

      scope.onEditorStateChanged(EditorState.Initial)
    }

  @Test
  fun `root Down eligibility and mapping distinguish header from document body`() =
    runTest(StandardTestDispatcher()) {
      val uiState =
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
            boundsInRoot = Rect(left = 0f, top = 120f, right = 400f, bottom = 920f),
            density = 1f,
          )
          updateEditorBounds(
            boundsInRoot = Rect(left = 40f, top = 200f, right = 360f, bottom = 680f),
            density = 1f,
          )
        }
      val scope = EditorInteractionScope(coroutineScope = this)
      scope.update(
        editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)),
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = uiState,
        density = 1f,
        visibleArea = EditorVisibleArea(),
        viewportState = EditorViewportState(),
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 320f),
        onSelectionHaptic = {},
        onRequestSoftwareKeyboard = {},
      )

      val headerPositionInRoot = Offset(x = 80f, y = 100f)
      val bodyPositionInRoot = Offset(x = 80f, y = 240f)

      // Down admission is decided in root coordinates. The cross-boundary
      // headerFieldPanClaimsViewportWithoutPromotingReleaseToFieldOrEditor layout test protects
      // this classification from being promoted after the pointer moves into the body.
      assertFalse(scope.containsDocumentInteraction(headerPositionInRoot))
      assertTrue(scope.containsDocumentInteraction(bodyPositionInRoot))
      assertFalse(
        scope.isTapEligible(headerPositionInRoot),
        "header root Down must not be document-eligible",
      )
      assertTrue(
        scope.isTapEligible(bodyPositionInRoot),
        "body root Down must remain document-eligible",
      )

      val bodyMapped = assertNotNull(scope.resolveInteractionPosition(bodyPositionInRoot))
      assertEquals(
        Offset(x = 40f, y = 40f),
        bodyMapped,
        "body mapping must subtract the editor rect top-left in root",
      )

      val headerMapped = assertNotNull(scope.resolveInteractionPosition(headerPositionInRoot))
      assertEquals(Offset(x = 40f, y = -100f), headerMapped)
      assertTrue(headerMapped.y < 0f, "mapping above the body must stay valid and negative")
    }

  @Test
  fun `outside-page tap eligibility follows document layout mode`() =
    runTest(StandardTestDispatcher()) {
      val uiState =
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
            boundsInRoot = Rect(left = 0f, top = 120f, right = 400f, bottom = 920f),
            density = 1f,
          )
          updateEditorBounds(
            boundsInRoot = Rect(left = 40f, top = 200f, right = 360f, bottom = 680f),
            density = 1f,
          )
        }
      val editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler))
      val scope = EditorInteractionScope(coroutineScope = this)
      val outsidePagePosition = Offset(x = 80f, y = 800f)

      scope.update(
        editor = editor,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = uiState,
        density = 1f,
        visibleArea = EditorVisibleArea(),
        viewportState = EditorViewportState(),
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 320f),
        onSelectionHaptic = {},
        onRequestSoftwareKeyboard = {},
      )
      assertTrue(scope.isTapEligible(outsidePagePosition))

      scope.update(
        editor = editor,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = uiState,
        density = 1f,
        visibleArea = EditorVisibleArea(),
        viewportState = EditorViewportState(),
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 320f,
            pageHeight = 480f,
            pageMarginTop = 20f,
            pageMarginBottom = 20f,
            pageMarginLeft = 20f,
            pageMarginRight = 20f,
          ),
        onSelectionHaptic = {},
        onRequestSoftwareKeyboard = {},
      )
      assertFalse(scope.isTapEligible(outsidePagePosition))
    }

  private fun updateScope(
    scope: EditorInteractionScope,
    editor: Editor,
    editing: () -> Boolean,
    uiState: EditorUiState = EditorUiState(),
    onRequestEditing: (Editor) -> Boolean = { false },
  ) {
    scope.update(
      editor = editor,
      bringIntoViewRequests = EditorBringIntoViewRequests(),
      uiState = uiState,
      density = 1f,
      visibleArea = EditorVisibleArea(),
      viewportState = EditorViewportState(),
      scrollGestureLockState = ScrollGestureLockState(),
      viewportZoomConfig = null,
      editing = editing,
      onRequestEditing = onRequestEditing,
      onSelectionHaptic = {},
      onRequestSoftwareKeyboard = {},
    )
  }
}

private fun EditorInteractionController.tap(pointerId: Long, position: Offset, downAtMillis: Long) {
  onPointerDown(
    change = pointerDown(id = pointerId, uptimeMillis = downAtMillis),
    position = position,
  )
  onPointerUp(
    change = pointerUp(id = pointerId, uptimeMillis = downAtMillis + 40L),
    position = position,
  )
}

private fun pointerDown(id: Long, uptimeMillis: Long): PointerInputChange =
  PointerInputChange(
    id = PointerId(id),
    uptimeMillis = uptimeMillis,
    position = Offset.Zero,
    pressed = true,
    previousUptimeMillis = uptimeMillis,
    previousPosition = Offset.Zero,
    previousPressed = false,
    isInitiallyConsumed = false,
  )

private fun pointerUp(id: Long, uptimeMillis: Long): PointerInputChange =
  PointerInputChange(
    id = PointerId(id),
    uptimeMillis = uptimeMillis,
    position = Offset.Zero,
    pressed = false,
    previousUptimeMillis = uptimeMillis,
    previousPosition = Offset.Zero,
    previousPressed = true,
    isInitiallyConsumed = false,
  )
