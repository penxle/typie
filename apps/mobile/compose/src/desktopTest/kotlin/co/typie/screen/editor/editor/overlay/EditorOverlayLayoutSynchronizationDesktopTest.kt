package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.PagePoint
import co.typie.editor.body.trackEditorContentBounds
import co.typie.editor.body.trackEditorInteractionSurfaceBounds
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.Size
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.TableOverlayCellSelection
import co.typie.editor.ffi.TableOverlayColumn
import co.typie.editor.ffi.TableOverlayRow
import co.typie.editor.interaction.EditorEdgeAutoScrollViewport
import co.typie.editor.interaction.EditorInteractionGeometry
import co.typie.editor.interaction.semantics.EditorTableColumnResizePresentation
import co.typie.editor.overlay.editorExtensionAreaLineHighlight
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.surface.editorPagePositionTracker
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.math.roundToInt
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorOverlayLayoutSynchronizationDesktopTest {
  @Test
  fun firstZoomedTableAxisPlacementUsesTheNewPagePosition() = runComposeUiTest {
    val zoom = mutableStateOf(1f)
    val uiState = focusedUiState()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor =
      Editor(
        FakeFfiEditor(
          selectionProvider = {
            val position = Position("cell-text", 0, Affinity.Downstream)
            Selection(anchor = position, head = position)
          },
          pageSizesProvider = {
            listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
          },
          tableOverlaysProvider = { listOf(tableOverlay()) },
        ),
        scope,
      )
    editor.sync {}
    mainClock.autoAdvance = false

    try {
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          Box(Modifier.size(RootSize.dp)) {
            Box(
              Modifier.offset {
                  val pageOffset = (PageOffsetAtZoomOne * zoom.value).roundToInt()
                  IntOffset(pageOffset, pageOffset)
                }
                .size(PageSizeAtZoomOne.dp * zoom.value)
                .testTag(PageTag)
                .editorPagePositionTracker(uiState = uiState, page = 1, density = 1f)
            )
            EditorTableAxisSelectionOverlay(
              editor = editor,
              uiState = uiState,
              editorRectInOverlay = { Rect(0f, 0f, RootSize, RootSize) },
              density = 1f,
              onTableAxisActionsRequest = { _, _ -> },
            )
          }
        }
      }
      mainClock.advanceTimeByFrame()

      runOnUiThread {
        uiState.updateDisplayZoom(2f)
        zoom.value = 2f
      }
      mainClock.advanceTimeByFrame()

      val pageBounds = onNodeWithTag(PageTag).fetchSemanticsNode().boundsInRoot
      val menuBounds =
        onNodeWithContentDescription(ColumnMenuDescription).fetchSemanticsNode().boundsInRoot
      assertEquals(58f, menuBounds.left - pageBounds.left, absoluteTolerance = 0.01f)
      assertEquals(31f, menuBounds.top - pageBounds.top, absoluteTolerance = 0.01f)

      runOnUiThread {
        uiState.updateDisplayZoom(1f)
        zoom.value = 1f
      }
      mainClock.advanceTimeByFrame()

      val zoomedOutPageBounds = onNodeWithTag(PageTag).fetchSemanticsNode().boundsInRoot
      val zoomedOutMenuBounds =
        onNodeWithContentDescription(ColumnMenuDescription).fetchSemanticsNode().boundsInRoot
      assertEquals(
        23f,
        zoomedOutMenuBounds.left - zoomedOutPageBounds.left,
        absoluteTolerance = 0.01f,
      )
      assertEquals(
        11f,
        zoomedOutMenuBounds.top - zoomedOutPageBounds.top,
        absoluteTolerance = 0.01f,
      )
    } finally {
      scope.cancel()
    }
  }

  @Test
  fun firstZoomedEditorBodyPlacementPublishesTheNewEditorBounds() = runComposeUiTest {
    val zoom = mutableStateOf(1f)
    val uiState = EditorUiState()
    mainClock.autoAdvance = false

    setContent {
      CompositionLocalProvider(LocalDensity provides Density(1f)) {
        Box(Modifier.size(RootSize.dp)) {
          Box(
            Modifier.offset {
                IntOffset(
                  InteractionSurfaceOffset.roundToInt(),
                  InteractionSurfaceOffset.roundToInt(),
                )
              }
              .size(InteractionSurfaceSize.dp)
              .testTag(InteractionSurfaceTag)
              .trackEditorInteractionSurfaceBounds(uiState = uiState, density = 1f)
          ) {
            Box(
              Modifier.offset {
                  val editorOffset = (EditorOffsetAtZoomOne * zoom.value).roundToInt()
                  IntOffset(editorOffset, editorOffset)
                }
                .size(EditorSizeAtZoomOne.dp * zoom.value)
                .testTag(EditorBoundsTag)
                .trackEditorContentBounds(uiState = uiState, density = 1f)
            )
            Box(
              Modifier.offset {
                  val editorBounds = uiState.editorBoundsInContainer
                  IntOffset(editorBounds.x.roundToInt(), editorBounds.y.roundToInt())
                }
                .size(EditorBoundsMarkerSize.dp)
                .testTag(EditorBoundsMarkerTag)
            )
          }
        }
      }
    }
    mainClock.advanceTimeByFrame()

    runOnUiThread { zoom.value = 2f }
    mainClock.advanceTimeByFrame()

    val editorBounds = onNodeWithTag(EditorBoundsTag).fetchSemanticsNode().boundsInRoot
    val markerBounds = onNodeWithTag(EditorBoundsMarkerTag).fetchSemanticsNode().boundsInRoot
    assertEquals(editorBounds.left, markerBounds.left, absoluteTolerance = 0.01f)
    assertEquals(editorBounds.top, markerBounds.top, absoluteTolerance = 0.01f)
  }

  @Test
  fun firstContinuousPageRelayoutDrawUsesTheNewPagePosition() = runComposeUiTest {
    val firstPageHeight = mutableStateOf(ContinuousPageHeight)
    val uiState = EditorUiState()
    val cursor =
      CursorMetrics(
        pageIdx = 1,
        caret = co.typie.editor.ffi.Rect(x = 0f, y = 10f, width = 1f, height = 10f),
        line = co.typie.editor.ffi.Rect(x = 0f, y = 10f, width = 100f, height = 10f),
      )
    mainClock.autoAdvance = false

    setContent {
      CompositionLocalProvider(
        LocalDensity provides Density(1f),
        LocalAppColors provides LightColors,
      ) {
        Box(
          Modifier.size(RootSize.dp)
            .background(Color.White)
            .testTag(RootTag)
            .trackEditorInteractionSurfaceBounds(uiState = uiState, density = 1f)
            .editorExtensionAreaLineHighlight(
              cursor = cursor,
              focused = true,
              editorBounds = { uiState.editorBoundsInContainer },
              viewportTransform = { uiState.resolveViewportTransform() },
              enabled = true,
              color = LightColors.surfaceInset.copy(alpha = 0.55f),
            )
        ) {
          Column(
            Modifier.offset { IntOffset(0, ContinuousEditorTop.roundToInt()) }
              .fillMaxWidth()
              .trackEditorContentBounds(uiState = uiState, density = 1f)
          ) {
            repeat(2) { page ->
              val pageHeight = if (page == 0) firstPageHeight.value else ContinuousPageHeight
              Box(
                Modifier.size(width = ContinuousPageWidth.dp, height = pageHeight.dp)
                  .editorPagePositionTracker(uiState = uiState, page = page, density = 1f)
              )
            }
          }
        }
      }
    }
    mainClock.advanceTimeByFrame()
    mainClock.advanceTimeByFrame()

    runOnUiThread { firstPageHeight.value = ContinuousRelayoutPageHeight }
    mainClock.advanceTimeByFrame()

    val pixels = onNodeWithTag(RootTag).captureToImage().toPixelMap()
    assertNotEquals(Color.White, pixels[ContinuousHighlightX, ContinuousRelayoutHighlightCenterY])
  }

  @Test
  fun firstZoomedSelectionHandleDrawUsesTheNewPagePosition() = runComposeUiTest {
    val zoom = mutableStateOf(1f)
    val selection =
      Selection(
        anchor = Position("text", 0, Affinity.Downstream),
        head = Position("text", 5, Affinity.Downstream),
      )
    val endpoints =
      SelectionEndpoints(
        from =
          PageRect(
            pageIdx = 1,
            rect = co.typie.editor.ffi.Rect(x = 10f, y = 20f, width = 4f, height = 8f),
          ),
        to =
          PageRect(
            pageIdx = 1,
            rect = co.typie.editor.ffi.Rect(x = 40f, y = 20f, width = 4f, height = 8f),
          ),
        fromPosition = selection.anchor,
        toPosition = selection.head,
      )
    val uiState = focusedUiState()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor =
      Editor(
        FakeFfiEditor(
          selectionProvider = { selection },
          selectionEndpointsProvider = { endpoints },
          pageSizesProvider = {
            listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
          },
        ),
        scope,
      )
    editor.sync {}
    mainClock.autoAdvance = false

    try {
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          Box(Modifier.size(RootSize.dp).background(Color.White).testTag(RootTag)) {
            Box(
              Modifier.offset {
                  val pageOffset = (PageOffsetAtZoomOne * zoom.value).roundToInt()
                  IntOffset(pageOffset, pageOffset)
                }
                .size(PageSizeAtZoomOne.dp * zoom.value)
                .editorPagePositionTracker(uiState = uiState, page = 1, density = 1f)
            )
            EditorSelectionHandleOverlay(editor = editor, uiState = uiState, density = 1f)
          }
        }
      }
      mainClock.advanceTimeByFrame()

      runOnUiThread {
        uiState.updateDisplayZoom(2f)
        zoom.value = 2f
      }
      mainClock.advanceTimeByFrame()

      val pixels = onNodeWithTag(RootTag).captureToImage().toPixelMap()
      assertEquals(LightColors.textDefault, pixels[ToHandleCenterX, ToHandleCenterY])
    } finally {
      scope.cancel()
    }
  }

  @Test
  fun firstZoomedTableCellHandleDrawUsesTheNewPagePosition() = runComposeUiTest {
    val zoom = mutableStateOf(1f)
    val uiState = focusedUiState()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor =
      Editor(
        FakeFfiEditor(
          selectionProvider = {
            val position = Position("cell-text", 0, Affinity.Downstream)
            Selection(anchor = position, head = position)
          },
          pageSizesProvider = {
            listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
          },
          tableOverlaysProvider = {
            listOf(
              tableOverlay(
                cellSelection =
                  TableOverlayCellSelection(anchorRow = 0, anchorCol = 0, headRow = 0, headCol = 0)
              )
            )
          },
        ),
        scope,
      )
    editor.sync {}
    mainClock.autoAdvance = false

    try {
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          Box(Modifier.size(RootSize.dp).background(Color.White).testTag(RootTag)) {
            Box(
              Modifier.offset {
                  val pageOffset = (PageOffsetAtZoomOne * zoom.value).roundToInt()
                  IntOffset(pageOffset, pageOffset)
                }
                .size(PageSizeAtZoomOne.dp * zoom.value)
                .editorPagePositionTracker(uiState = uiState, page = 1, density = 1f)
            )
            EditorTableCellSelectionOverlay(editor = editor, uiState = uiState, density = 1f)
          }
        }
      }
      mainClock.advanceTimeByFrame()

      runOnUiThread {
        uiState.updateDisplayZoom(2f)
        zoom.value = 2f
      }
      mainClock.advanceTimeByFrame()

      val pixels = onNodeWithTag(RootTag).captureToImage().toPixelMap()
      assertEquals(LightColors.textDefault, pixels[TableCellHandleCenter, TableCellHandleCenter])
    } finally {
      scope.cancel()
    }
  }

  @Test
  fun firstZoomedTableColumnResizeDrawUsesTheNewPagePosition() = runComposeUiTest {
    val zoom = mutableStateOf(1f)
    val uiState = focusedUiState()
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    val editor =
      Editor(
        FakeFfiEditor(
          selectionProvider = {
            val position = Position("cell-text", 0, Affinity.Downstream)
            Selection(anchor = position, head = position)
          },
          pageSizesProvider = {
            listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f))
          },
          tableOverlaysProvider = { listOf(tableOverlay()) },
        ),
        scope,
      )
    val interactionGeometry = TestInteractionGeometry(editor = editor, uiState = uiState)
    editor.sync {}
    mainClock.autoAdvance = false

    try {
      setContent {
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalAppColors provides LightColors,
          LocalAppShadows provides LightAppShadows,
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          Box(Modifier.size(RootSize.dp).background(Color.White).testTag(RootTag)) {
            Box(
              Modifier.offset {
                  val pageOffset = (PageOffsetAtZoomOne * zoom.value).roundToInt()
                  IntOffset(pageOffset, pageOffset)
                }
                .size(PageSizeAtZoomOne.dp * zoom.value)
                .editorPagePositionTracker(uiState = uiState, page = 1, density = 1f)
            )
            EditorTableColumnResizeOverlay(
              editor = editor,
              uiState = uiState,
              geometry = interactionGeometry,
              presentation = EditorTableColumnResizePresentation(pressed = false, draft = null),
            )
          }
        }
      }
      mainClock.advanceTimeByFrame()

      runOnUiThread {
        uiState.updateDisplayZoom(2f)
        zoom.value = 2f
      }
      mainClock.advanceTimeByFrame()

      val pixels = onNodeWithTag(RootTag).captureToImage().toPixelMap()
      assertNotEquals(Color.White, pixels[TableColumnResizeCenterX, TableColumnResizeCenterY])
    } finally {
      scope.cancel()
    }
  }

  private fun tableOverlay(cellSelection: TableOverlayCellSelection? = null): TableOverlay =
    TableOverlay(
      tableId = "table",
      pageIdx = 1,
      bounds = co.typie.editor.ffi.Rect(x = 10f, y = 20f, width = 100f, height = 80f),
      borderStyle = TableBorderStyle.Solid,
      align = Alignment.Left,
      proportion = 1f,
      contentWidth = 100f,
      minProportionWidth = 83f,
      maxProportionWidth = 100f,
      rows = listOf(TableOverlayRow(index = 0, height = 40f, position = 40f)),
      columns = listOf(TableOverlayColumn(index = 0, widthAsPx = 50f, position = 50f)),
      rowCount = 1,
      isLastRowFragment = true,
      isFocused = true,
      focusedRowIndex = 0,
      focusedColIndex = 0,
      cellSelection = cellSelection,
    )

  private fun focusedUiState(): EditorUiState =
    EditorUiState().apply {
      updateFocus(true)
      updateDisplayZoom(1f)
      val editorBounds = Rect(0f, 0f, RootSize, RootSize)
      updateInteractionSurfaceBounds(boundsInRoot = editorBounds, density = 1f)
      updateEditorBounds(boundsInRoot = editorBounds, density = 1f)
    }

  private class TestInteractionGeometry(
    private val editor: Editor,
    private val uiState: EditorUiState,
  ) : EditorInteractionGeometry {
    override val density = 1f

    override fun containsDocumentInteraction(positionInRoot: Offset): Boolean = true

    override fun resolveInteractionPosition(positionInRoot: Offset): Offset = positionInRoot

    override fun isTapEligible(positionInRoot: Offset): Boolean = true

    override fun resolvePoint(positionInNode: Offset): PagePoint? = null

    override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? =
      uiState.resolveViewportTransform(editor.pageSizes).localToGlobal(page = page, x = x, y = y)

    override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? = null
  }

  private companion object {
    const val ColumnMenuDescription = "열 메뉴"
    const val ContinuousEditorTop = 40f
    const val ContinuousHighlightX = 10
    const val ContinuousPageHeight = 100f
    const val ContinuousPageWidth = 100f
    const val ContinuousRelayoutHighlightCenterY = 255
    const val ContinuousRelayoutPageHeight = 200f
    const val EditorBoundsMarkerSize = 10f
    const val EditorBoundsMarkerTag = "editor-bounds-marker"
    const val EditorBoundsTag = "zoomed-editor-bounds"
    const val EditorOffsetAtZoomOne = 40f
    const val EditorSizeAtZoomOne = 100f
    const val InteractionSurfaceOffset = 30f
    const val InteractionSurfaceSize = 400f
    const val InteractionSurfaceTag = "interaction-surface"
    const val PageTag = "zoomed-page"
    const val RootTag = "overlay-root"
    const val PageOffsetAtZoomOne = 100f
    const val PageSizeAtZoomOne = 100f
    const val RootSize = 500f
    const val TableCellHandleCenter = 320
    const val TableColumnResizeCenterX = 320
    const val TableColumnResizeCenterY = 300
    const val ToHandleCenterX = 281
    const val ToHandleCenterY = 264
  }
}
