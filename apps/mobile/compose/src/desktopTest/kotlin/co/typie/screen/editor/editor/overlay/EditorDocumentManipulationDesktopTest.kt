package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.gestures.Scrollable2DState
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect as ComposeRect
import androidx.compose.ui.geometry.Size as ComposeSize
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollDispatcher
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SkikoComposeUiTest
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.FakeFfiEditor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOp
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.EditorScreenPointerSequence
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.editorInteractions
import co.typie.editor.interaction.observeEditorScreenPointerSequence
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorRuntime
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.consumeEditorViewportTouchPan
import co.typie.ext.ScrollGestureLockState
import co.typie.platform.Platform
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher

@OptIn(ExperimentalTestApi::class, InternalComposeUiApi::class, ExperimentalCoroutinesApi::class)
class EditorDocumentManipulationDesktopTest {
  @Test
  fun selectionHandleAndPagePointersStartOnePinch() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val pageSizes = listOf(Size(width = 120f, height = 120f))
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 120f,
        pageHeight = 120f,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { pageSizes },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updateDisplayZoom(1f)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    val viewportState = EditorViewportState()
    val zoomController =
      EditorZoomController().apply { syncLayout(layoutSpec = layoutSpec, viewportWidth = 120f) }
    val viewportZoomConfig =
      EditorViewportZoomSemanticConfig(
        layoutSpec = layoutSpec,
        zoomController = zoomController,
        viewportState = viewportState,
        uiState = uiState,
        pageSizes = pageSizes,
        viewportWidth = 120f,
        density = 1f,
        onZoomSnap = {},
      )
    lateinit var interactionScope: EditorInteractionScope
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        viewportState = viewportState,
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        viewportZoomConfig = viewportZoomConfig,
        onInteractionScope = { interactionScope = it },
      )
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 0, position = Offset(x = 42f, y = 30f))
        down(pointerId = 1, position = Offset(x = 90f, y = 90f))
      }
      waitForIdle()

      assertEquals(
        EditorInteractionMode.ViewportZooming,
        interactionScope.controller.interactionMode,
      )

      onNodeWithTag(RootTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableCellHandleAndPagePointersStartOnePinch() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val pageSizes = listOf(Size(width = 120f, height = 120f))
    val layoutSpec = testPaginatedLayoutSpec()
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { pageSizes },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState = focusedTestUiState()
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    val viewportState = EditorViewportState()
    val viewportZoomConfig =
      testViewportZoomConfig(
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        uiState = uiState,
        viewportState = viewportState,
      )
    lateinit var interactionScope: EditorInteractionScope
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        viewportState = viewportState,
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        viewportZoomConfig = viewportZoomConfig,
        onInteractionScope = { interactionScope = it },
      )
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 0, position = Offset(x = 60f, y = 60f))
        down(pointerId = 1, position = Offset(x = 100f, y = 20f))
      }
      waitForIdle()

      assertEquals(
        EditorInteractionMode.ViewportZooming,
        interactionScope.controller.interactionMode,
      )

      onNodeWithTag(RootTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun pinchTakeoverCancelsColumnResizeWithoutCommit() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val pageSizes = listOf(Size(width = 120f, height = 120f))
    val layoutSpec = testPaginatedLayoutSpec()
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { pageSizes },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState = focusedTestUiState()
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    val viewportState = EditorViewportState()
    val viewportZoomConfig =
      testViewportZoomConfig(
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        uiState = uiState,
        viewportState = viewportState,
      )
    lateinit var interactionScope: EditorInteractionScope
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        viewportState = viewportState,
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        viewportZoomConfig = viewportZoomConfig,
        onInteractionScope = { interactionScope = it },
      )
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(pointerId = 0, position = Offset(x = 60f, y = 30f))
        moveTo(pointerId = 0, position = Offset(x = 80f, y = 30f))
        down(pointerId = 1, position = Offset(x = 100f, y = 100f))
      }
      waitForIdle()

      assertEquals(
        EditorInteractionMode.ViewportZooming,
        interactionScope.controller.interactionMode,
      )
      assertTrue(fake.enqueued.filterIsInstance<Message.Node>().isEmpty())

      onNodeWithTag(RootTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
      waitForIdle()
      assertTrue(fake.enqueued.filterIsInstance<Message.Node>().isEmpty())
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun selectionHandleRoutesDragBeforeLowerPointerOverlay() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 42f, y = 30f))
        moveTo(Offset(x = 52f, y = 50f))
      }
      waitForIdle()

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(endpoints.fromPosition, extend.anchor)
      assertEquals(50f, extend.headX)
      assertEquals(44f, extend.headY)
      assertFalse(extend.allowCollapse)
      assertNull(extend.baseSelection)
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun paginatedSelectionHandleUsesUnclampedCoordinatesOutsidePageBounds() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 0f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val pageSizes = listOf(Size(width = 80f, height = 120f))
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { pageSizes },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(
          ComposeRect(
            offset = Offset(x = 40f, y = 0f),
            size = ComposeSize(width = 80f, height = 120f),
          ),
          density = 1f,
        )
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 80f,
            pageHeight = 120f,
            pageMarginTop = 0f,
            pageMarginBottom = 0f,
            pageMarginLeft = 0f,
            pageMarginRight = 0f,
          ),
        pageSizes = pageSizes,
      )
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 0f, y = 20f))
        moveTo(Offset(x = 10f, y = 40f))
        up()
      }
      waitForIdle()
      assertTrue(fake.enqueued.filterIsInstance<Message.Selection>().isEmpty())

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 20f, y = 20f))
        moveTo(Offset(x = 30f, y = 40f))
        up()
      }
      waitForIdle()

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(endpoints.toPosition, extend.anchor)
      assertEquals(10f, extend.headX)
      assertEquals(44f, extend.headY)
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun selectionHandleAreaWheelScrollsViewport() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { listOf(Size(width = 120f, height = 400f)) },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(
          ComposeRect(Offset.Zero, ComposeSize(width = 120f, height = 400f)),
          density = 1f,
        )
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    val viewportState = EditorViewportState()
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        viewportState = viewportState,
        viewportContentSize = ComposeSize(width = 120f, height = 400f),
      )
      waitForIdle()

      onNodeWithTag(RootTag).performMouseInput {
        moveTo(Offset(x = 42f, y = 30f))
        scroll(Offset(x = 0f, y = 120f))
      }
      waitForIdle()

      assertTrue(viewportState.scrollOffset.y > 0f)
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun selectionHandleAreaModifiedWheelZoomsViewport() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val pageSizes = listOf(Size(width = 120f, height = 120f))
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 120f,
        pageHeight = 120f,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { pageSizes },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updateDisplayZoom(1f)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(
          ComposeRect(Offset.Zero, ComposeSize(width = 120f, height = 120f)),
          density = 1f,
        )
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    val viewportState = EditorViewportState()
    val zoomController =
      EditorZoomController().apply { syncLayout(layoutSpec = layoutSpec, viewportWidth = 120f) }
    val viewportZoomConfig =
      EditorViewportZoomSemanticConfig(
        layoutSpec = layoutSpec,
        zoomController = zoomController,
        viewportState = viewportState,
        uiState = uiState,
        pageSizes = pageSizes,
        viewportWidth = 120f,
        density = 1f,
        onZoomSnap = {},
      )
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        viewportState = viewportState,
        viewportContentSize = ComposeSize(width = 120f, height = 400f),
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        viewportZoomConfig = viewportZoomConfig,
      )
      waitForIdle()

      val previousZoom = zoomController.displayZoom
      val scene = (this as SkikoComposeUiTest).scene
      runOnUiThread {
        scene.sendPointerEvent(
          eventType = PointerEventType.Scroll,
          position = Offset(x = 42f, y = 30f),
          scrollDelta = Offset(x = 0f, y = -12f),
          keyboardModifiers = PointerKeyboardModifiers(isCtrlPressed = true),
        )
      }
      waitForIdle()

      assertTrue(zoomController.displayZoom > previousZoom)
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun documentHandleVisualsDoNotBlockPointerInputOutsideHandleTargets() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
        to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
        fromPosition = Position("text", 0, Affinity.Downstream),
        toPosition = Position("text", 5, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        selectionEndpointsProvider = { endpoints },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    var lowerPointerDownCount = 0
    editor.sync {}
    try {
      setOverlayHostContent(
        editor = editor,
        runtime = runtime,
        uiState = uiState,
        scope = scope,
        onLowerPointerDown = { lowerPointerDownCount += 1 },
      )
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 90f, y = 10f))
        up()
      }
      waitForIdle()

      assertEquals(1, lowerPointerDownCount)
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun selectionHandleDragHandsOffToTableCellHandleAfterCellSelectionAppears() =
    runComposeUiTest test@{
      val endpoints =
        SelectionEndpoints(
          from = PageRect(pageIdx = 0, rect = Rect(x = 10f, y = 20f, width = 4f, height = 8f)),
          to = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 20f, width = 4f, height = 8f)),
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
          pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
          tableOverlaysProvider = { listOf(tableOverlay) },
        )
      val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
      val editor = Editor(fake, scope)
      val uiState =
        EditorUiState().apply {
          updateFocus(true)
          updatePageOffset(page = 0, offset = Offset.Zero)
          updateInteractionSurfaceBounds(TestRootRect, density = 1f)
          updateEditorBounds(TestRootRect, density = 1f)
        }
      val runtime = EditorRuntime(scope).apply { attach(editor) }
      editor.sync {}
      try {
        setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
        waitForIdle()

        val root = onNodeWithTag(RootTag)
        root.performTouchInput {
          down(Offset(x = 42f, y = 30f))
          moveTo(Offset(x = 52f, y = 50f))
        }
        waitForIdle()

        tableOverlay =
          tableOverlay(
            isFocused = true,
            cellSelection =
              TableOverlayCellSelection(anchorRow = 0, anchorCol = 0, headRow = 0, headCol = 1),
          )
        editor.sync {}
        fake.enqueued.clear()
        waitForIdle()

        root.performTouchInput {
          moveTo(Offset(x = 64f, y = 60f))
          up()
        }
        waitForIdle()

        val extend =
          fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
        assertEquals(selection.anchor, extend.anchor)
        assertEquals(selection, extend.baseSelection)
        assertEquals(62f, extend.headX)
        assertEquals(54f, extend.headY)
        assertFalse(extend.allowCollapse)
      } finally {
        runtime.clear(editor)
        scope.cancel()
      }
    }

  @Test
  fun tableColumnResizeHandleTapDispatchesNormalEditorTap() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 60f, y = 30f))
        up()
      }
      waitUntil { fake.enqueued.filterIsInstance<Message.Selection>().isNotEmpty() }

      assertEquals(
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 60f, y = 30f))),
        fake.enqueued.filterIsInstance<Message.Selection>(),
      )
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableColumnResizeHandleHoldDoesNotDispatchTapBeforePointerUp() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val dispatcher = StandardTestDispatcher()
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 60f, y = 30f))
        dispatcher.scheduler.advanceTimeBy(400L)
        dispatcher.scheduler.runCurrent()
        assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())
        up()
      }
      dispatcher.scheduler.advanceUntilIdle()
      waitUntil { fake.enqueued.filterIsInstance<Message.Selection>().isNotEmpty() }

      assertEquals(
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 60f, y = 30f))),
        fake.enqueued.filterIsInstance<Message.Selection>(),
      )
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableColumnResizeHandleDownClearsTapHistoryBeforeDoubleTapDispatch() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val dispatcher = StandardTestDispatcher()
    val scope = CoroutineScope(SupervisorJob() + dispatcher)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      val root = onNodeWithTag(RootTag)
      root.performTouchInput {
        down(Offset(x = 60f, y = 30f))
        up()
      }
      dispatcher.scheduler.advanceUntilIdle()
      waitUntil { fake.enqueued.filterIsInstance<Message.Selection>().isNotEmpty() }
      fake.enqueued.clear()

      root.performTouchInput { down(Offset(x = 60f, y = 30f)) }
      dispatcher.scheduler.runCurrent()

      assertEquals(emptyList(), fake.enqueued.filterIsInstance<Message.Selection>())

      root.performTouchInput { up() }
      dispatcher.scheduler.advanceUntilIdle()
      waitUntil { fake.enqueued.filterIsInstance<Message.Selection>().isNotEmpty() }

      assertEquals(
        listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 60f, y = 30f))),
        fake.enqueued.filterIsInstance<Message.Selection>(),
      )
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableColumnResizeHandleDragResizesColumn() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(
          ComposeRect(
            offset = Offset(x = 20f, y = 20f),
            size = ComposeSize(width = 120f, height = 120f),
          ),
          density = 1f,
        )
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 80f, y = 50f))
        moveTo(Offset(x = 100f, y = 50f))
        up()
      }
      waitUntil { fake.enqueued.filterIsInstance<Message.Node>().isNotEmpty() }

      assertEquals(
        listOf(
          Message.Node(
            NodeOp.Table(id = "table", op = TableOp.SetColumnWidths(widths = listOf(0.6f, 0.4f)))
          )
        ),
        fake.enqueued.filterIsInstance<Message.Node>(),
      )
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableColumnResizeHandleDragStartsWithinHandleHitArea() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 60f, y = 30f))
        moveTo(Offset(x = 70f, y = 30f))
        up()
      }
      waitUntil { fake.enqueued.filterIsInstance<Message.Node>().isNotEmpty() }

      assertEquals(
        listOf(
          Message.Node(
            NodeOp.Table(id = "table", op = TableOp.SetColumnWidths(widths = listOf(0.6f, 0.4f)))
          )
        ),
        fake.enqueued.filterIsInstance<Message.Node>(),
      )
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableColumnResizeHandleDragIncludesPreSlopMovement() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 60f, y = 30f))
        moveTo(Offset(x = 61f, y = 30f))
        moveTo(Offset(x = 80f, y = 30f))
        up()
      }
      waitUntil { fake.enqueued.filterIsInstance<Message.Node>().isNotEmpty() }

      assertEquals(
        listOf(
          Message.Node(
            NodeOp.Table(id = "table", op = TableOp.SetColumnWidths(widths = listOf(0.6f, 0.4f)))
          )
        ),
        fake.enqueued.filterIsInstance<Message.Node>(),
      )
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  @Test
  fun tableCellHandleRoutesDragBeforeLowerPointerOverlay() = runComposeUiTest {
    val selection =
      Selection(
        anchor = Position("cell-text", 0, Affinity.Downstream),
        head = Position("cell-text", 0, Affinity.Downstream),
      )
    val fake =
      FakeFfiEditor(
        selectionProvider = { selection },
        pageSizesProvider = { listOf(Size(width = 120f, height = 120f)) },
        tableOverlaysProvider = {
          listOf(tableOverlay(isFocused = true, focusedRowIndex = 0, focusedColIndex = 0))
        },
      )
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor = Editor(fake, scope)
    val uiState =
      EditorUiState().apply {
        updateFocus(true)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(TestRootRect, density = 1f)
        updateEditorBounds(TestRootRect, density = 1f)
      }
    val runtime = EditorRuntime(scope).apply { attach(editor) }
    editor.sync {}
    try {
      setOverlayHostContent(editor = editor, runtime = runtime, uiState = uiState, scope = scope)
      waitForIdle()

      onNodeWithTag(RootTag).performTouchInput {
        down(Offset(x = 60f, y = 60f))
        moveTo(Offset(x = 100f, y = 90f))
        up()
      }
      waitForIdle()

      val extend =
        fake.enqueued.filterIsInstance<Message.Selection>().single().op as SelectionOp.ExtendTo
      assertEquals(selection.anchor, extend.anchor)
      assertEquals(100f, extend.headX)
      assertEquals(90f, extend.headY)
      assertEquals(selection, extend.baseSelection)
      assertFalse(extend.allowCollapse)
    } finally {
      runtime.clear(editor)
      scope.cancel()
    }
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setOverlayHostContent(
    editor: Editor,
    runtime: EditorRuntime,
    uiState: EditorUiState,
    scope: CoroutineScope,
    viewportState: EditorViewportState = EditorViewportState(),
    viewportContentSize: ComposeSize = TestRootSize,
    layoutSpec: EditorDocumentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 120f),
    pageSizes: List<Size> = listOf(Size(width = 120f, height = 120f)),
    displayZoom: Float = 1f,
    viewportZoomConfig: EditorViewportZoomSemanticConfig? = null,
    onLowerPointerDown: () -> Unit = {},
    onInteractionScope: (EditorInteractionScope) -> Unit = {},
  ) {
    setContent {
      val interactionScope = remember {
        EditorInteractionScope(coroutineScope = scope, platformProvider = { Platform.Desktop })
      }
      val rememberedViewportState = remember { viewportState }
      val visibleArea = remember { EditorVisibleArea(viewport = TestRootSize) }
      val bringIntoViewRequests = remember { EditorBringIntoViewRequests() }
      val scrollGestureLockState = remember { ScrollGestureLockState() }
      val screenPointerSequence = remember { EditorScreenPointerSequence() }
      val nestedScrollDispatcher = remember { NestedScrollDispatcher() }
      val scrollableState =
        remember(rememberedViewportState) {
          Scrollable2DState { delta ->
            consumeEditorViewportTouchPan(
              viewportState = rememberedViewportState,
              deltaPx = delta,
              density = 1f,
            )
          }
        }

      SideEffect {
        rememberedViewportState.updateMeasuredBounds(
          viewportSize = TestRootSize,
          contentSize = viewportContentSize,
        )
      }
      SideEffect {
        onInteractionScope(interactionScope)
        interactionScope.update(
          editor = editor,
          bringIntoViewRequests = bringIntoViewRequests,
          uiState = uiState,
          visibleArea = visibleArea,
          viewportState = rememberedViewportState,
          density = 1f,
          scrollGestureLockState = scrollGestureLockState,
          viewportZoomConfig = viewportZoomConfig,
          layoutSpec = layoutSpec,
          onSelectionHaptic = {},
          onRequestSoftwareKeyboard = {},
        )
        interactionScope.onEditorStateChanged(editor.state)
      }

      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalEditorRuntime provides runtime,
        LocalEditorUiState provides uiState,
        LocalEditorBringIntoViewRequests provides bringIntoViewRequests,
        LocalEditorInteractionScope provides interactionScope,
      ) {
        Box(
          Modifier.testTag(RootTag)
            .size(120.dp)
            .observeEditorScreenPointerSequence(screenPointerSequence)
        ) {
          Box(
            Modifier.fillMaxSize()
              .nestedScroll(object : NestedScrollConnection {}, nestedScrollDispatcher)
              .editorInteractions(
                interactionController = interactionScope.controller,
                geometry = interactionScope,
                screenPointerSequence = screenPointerSequence,
                scrollableState = scrollableState,
                nestedScrollDispatcher = nestedScrollDispatcher,
                touchSlop = 8f,
                density = 1f,
              )
          ) {
            LowerPointerOverlay(onPointerDown = onLowerPointerDown)
            EditorTableColumnResizeOverlay(
              editor = editor,
              uiState = uiState,
              geometry = interactionScope,
              editorOffsetInSurface =
                uiState.editorBoundsInContainer.let { bounds -> Offset(bounds.x, bounds.y) },
              presentation = interactionScope.controller.tableColumnResizePresentation,
            )
            EditorTableCellSelectionOverlay(
              editor = editor,
              uiState = uiState,
              editorRectInOverlay = uiState.editorRectInSurface(),
              density = 1f,
            )
            EditorSelectionHandleOverlay(
              editor = editor,
              uiState = uiState,
              editorRectInOverlay = uiState.editorRectInSurface(),
              density = 1f,
            )
          }
        }
      }
    }
  }

  private fun focusedTestUiState(): EditorUiState =
    EditorUiState().apply {
      updateFocus(true)
      updateDisplayZoom(1f)
      updatePageOffset(page = 0, offset = Offset.Zero)
      updateInteractionSurfaceBounds(TestRootRect, density = 1f)
      updateEditorBounds(TestRootRect, density = 1f)
    }

  private fun EditorUiState.editorRectInSurface(): ComposeRect =
    editorBoundsInContainer.let { bounds ->
      ComposeRect(
        left = bounds.x,
        top = bounds.y,
        right = bounds.x + bounds.width,
        bottom = bounds.y + bounds.height,
      )
    }

  private fun testPaginatedLayoutSpec(): EditorDocumentLayoutSpec.Paginated =
    EditorDocumentLayoutSpec.Paginated(
      pageWidth = 120f,
      pageHeight = 120f,
      pageMarginTop = 0f,
      pageMarginBottom = 0f,
      pageMarginLeft = 0f,
      pageMarginRight = 0f,
    )

  private fun testViewportZoomConfig(
    layoutSpec: EditorDocumentLayoutSpec.Paginated,
    pageSizes: List<Size>,
    uiState: EditorUiState,
    viewportState: EditorViewportState,
  ): EditorViewportZoomSemanticConfig {
    val zoomController =
      EditorZoomController().apply {
        syncLayout(layoutSpec = layoutSpec, viewportWidth = TestRootSize.width)
      }
    return EditorViewportZoomSemanticConfig(
      layoutSpec = layoutSpec,
      zoomController = zoomController,
      viewportState = viewportState,
      uiState = uiState,
      pageSizes = pageSizes,
      viewportWidth = TestRootSize.width,
      density = 1f,
      onZoomSnap = {},
    )
  }

  @Composable
  private fun LowerPointerOverlay(onPointerDown: () -> Unit) {
    Box(
      Modifier.fillMaxSize().pointerInput(Unit) {
        awaitEachGesture {
          awaitFirstDown(requireUnconsumed = false)
          onPointerDown()
          while (true) {
            val event = awaitPointerEvent()
            val change = event.changes.first()
            if (change.changedToUp()) {
              change.consume()
              break
            }
            change.consume()
          }
        }
      }
    )
  }

  private companion object {
    const val RootTag = "editor-document-manipulation-root"
    val TestRootSize = ComposeSize(width = 120f, height = 120f)
    val TestRootRect = ComposeRect(Offset.Zero, TestRootSize)

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
  }
}
