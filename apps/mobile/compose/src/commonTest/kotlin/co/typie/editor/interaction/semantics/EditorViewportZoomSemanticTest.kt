package co.typie.editor.interaction.semantics

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import co.typie.editor.EditorZoomController
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.EditorPinchSample
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import kotlin.math.ln
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EditorViewportZoomSemanticTest {
  @Test
  fun `pinch zoom keeps the anchor under the focal point`() {
    val fixture = Fixture()

    val startSample = EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f)
    assertTrue(fixture.semantic.beginPinch(startSample))
    assertTrue(fixture.viewportState.isTransforming)
    assertTrue(fixture.semantic.updatePinch(startSample.copy(distancePx = 150f)))

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.end()
    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `pinch samples resolve an absolute target from a root-stable focal`() {
    val fixture =
      Fixture(
        pageSizes =
          listOf(PageSize(width = 720f, height = 200f), PageSize(width = 720f, height = 300f)),
        viewportWidth = 1000f,
        initialScrollOffset = Offset(200f, 50f),
        pageOffsets = mapOf(0 to Offset(140f, 0f), 1 to Offset(140f, 224f)),
        editorBoundsInRoot = Rect(left = 100f, top = 200f, right = 1100f, bottom = 1200f),
      )
    val startSample = EditorPinchSample(focalInRootPx = Offset(340f, 474f), distancePx = 100f)

    assertTrue(fixture.semantic.beginPinch(startSample))
    assertTrue(fixture.semantic.updatePinch(startSample.copy(distancePx = 150f)))
    assertEquals(Offset(110f, 187f), fixture.viewportState.scrollOffset)
    assertEquals(2, fixture.viewportState.lastScrollRevision)

    assertTrue(fixture.semantic.updatePinch(startSample.copy(distancePx = 150f)))
    assertEquals(Offset(110f, 187f), fixture.viewportState.scrollOffset)
    assertEquals(2, fixture.viewportState.lastScrollRevision)

    fixture.updateEditorRootOffset(Offset(80f, 160f))
    assertTrue(fixture.semantic.updatePinch(startSample.copy(distancePx = 150f)))
    assertEquals(Offset(110f, 187f), fixture.viewportState.scrollOffset)
    assertEquals(2, fixture.viewportState.lastScrollRevision)

    val movedSample =
      startSample.copy(
        focalInRootPx = startSample.focalInRootPx + Offset(20f, 10f),
        distancePx = 150f,
      )
    assertTrue(fixture.semantic.updatePinch(movedSample))
    assertEquals(Offset(90f, 177f), fixture.viewportState.scrollOffset)
    assertEquals(3, fixture.viewportState.lastScrollRevision)

    assertTrue(fixture.semantic.updatePinch(movedSample))
    assertEquals(Offset(90f, 177f), fixture.viewportState.scrollOffset)
    assertEquals(3, fixture.viewportState.lastScrollRevision)
  }

  @Test
  fun `pinch anchor follows the actual page width inside the layout track`() {
    val fixture =
      Fixture(
        pageSizes = listOf(PageSize(width = 700f, height = 960f)),
        viewportWidth = 960f,
        initialScrollOffset = Offset(100f, 0f),
        pageOffsets = mapOf(0 to Offset(10f, 0f)),
        editorBoundsInRoot = Rect(left = 20f, top = 0f, right = 740f, bottom = 2000f),
      )
    val startSample = EditorPinchSample(focalInRootPx = Offset(130f, 200f), distancePx = 100f)

    assertTrue(fixture.semantic.beginPinch(startSample))
    assertTrue(fixture.semantic.updatePinch(startSample.copy(distancePx = 150f)))

    assertEquals(Offset(35f, 100f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `focal-only pinch update does not wait for measured bounds`() {
    val fixture = Fixture(contentSize = Size(width = 200f, height = 200f))

    val startSample = EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f)
    assertTrue(fixture.semantic.beginPinch(startSample))
    assertTrue(fixture.semantic.updatePinch(startSample.copy(focalInRootPx = Offset(-100f, -100f))))
    assertEquals(1f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(x = 100f, y = 80f), fixture.viewportState.scrollOffset)

    fixture.semantic.end()
    fixture.viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 100f, height = 120f),
      contentSize = Size(width = 500f, height = 500f),
    )

    assertEquals(Offset(x = 100f, y = 80f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `pointer signal zoom shares the viewport zoom semantic`() {
    val fixture = Fixture()
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginPointerSignal())
    assertTrue(fixture.viewportState.isTransforming)
    assertTrue(
      fixture.semantic.updatePointerSignal(
        focalInEditorPx = Offset(80f, 150f),
        normalizedDelta = normalizedDeltaForOneAndHalfZoom,
      )
    )

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.end()
    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `pointer signal anchor follows the actual page width inside the layout track`() {
    val fixture =
      Fixture(
        pageSizes = listOf(PageSize(width = 700f, height = 960f)),
        viewportWidth = 960f,
        measuredViewportSize = Size(width = 960f, height = 900f),
        contentSize = Size(width = 1080f, height = 2000f),
        pageOffsets = mapOf(0 to Offset(10f, 0f)),
      )
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginPointerSignal())
    assertTrue(
      fixture.semantic.updatePointerSignal(Offset(310f, 0f), normalizedDeltaForOneAndHalfZoom)
    )

    assertEquals(Offset(35f, 0f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `pointer signal target is restored after zoom bounds are measured`() {
    val fixture =
      Fixture(
        measuredViewportSize = Size(width = 720f, height = 900f),
        contentSize = Size(width = 720f, height = 2000f),
      )
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginPointerSignal())
    assertTrue(
      fixture.semantic.updatePointerSignal(Offset(300f, 0f), normalizedDeltaForOneAndHalfZoom)
    )
    assertEquals(Offset.Zero, fixture.viewportState.scrollOffset)

    fixture.viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 720f, height = 900f),
      contentSize = Size(width = 1080f, height = 2000f),
    )

    assertEquals(Offset(150f, 0f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `pointer signal updates remain cumulative before zoom bounds are measured`() {
    val fixture =
      Fixture(
        measuredViewportSize = Size(width = 720f, height = 900f),
        contentSize = Size(width = 720f, height = 2000f),
      )
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginPointerSignal())
    assertTrue(
      fixture.semantic.updatePointerSignal(Offset(300f, 0f), normalizedDeltaForOneAndHalfZoom)
    )
    assertTrue(
      fixture.semantic.updatePointerSignal(Offset(300f, 0f), normalizedDeltaForOneAndHalfZoom)
    )
    assertEquals(2f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset.Zero, fixture.viewportState.scrollOffset)

    fixture.viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 720f, height = 900f),
      contentSize = Size(width = 1440f, height = 2000f),
    )

    assertEquals(Offset(300f, 0f), fixture.viewportState.scrollOffset)
  }

  private class Fixture(
    val pageSizes: List<PageSize> = listOf(PageSize(width = 720f, height = 960f)),
    val viewportWidth: Float = 720f,
    initialScrollOffset: Offset = Offset.Zero,
    pageOffsets: Map<Int, Offset> = mapOf(0 to Offset.Zero),
    editorBoundsInRoot: Rect = Rect(left = 0f, top = 0f, right = 720f, bottom = 2000f),
    measuredViewportSize: Size = Size(width = 100f, height = 120f),
    contentSize: Size = Size(width = 2000f, height = 2000f),
  ) {
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    val zoomController = EditorZoomController()
    val viewportState =
      EditorViewportState().apply {
        updateMeasuredBounds(viewportSize = measuredViewportSize, contentSize = contentSize)
        scrollTo(initialScrollOffset)
      }
    val uiState =
      EditorUiState().apply {
        updateDisplayZoom(1f)
        pageOffsets.forEach { (page, offset) -> updatePageOffset(page = page, offset = offset) }
        updateEditorBounds(boundsInRoot = editorBoundsInRoot, density = 1f)
      }
    val semantic =
      EditorViewportZoomSemantic().apply {
        configure(
          EditorViewportZoomSemanticConfig(
            layoutSpec = layoutSpec,
            zoomController = zoomController,
            viewportState = viewportState,
            uiState = uiState,
            pageSizes = pageSizes,
            viewportWidth = viewportWidth,
            density = 1f,
            onZoomSnap = {},
          )
        )
      }

    init {
      zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = viewportWidth)
    }

    fun updateEditorRootOffset(offset: Offset) {
      uiState.updateEditorBounds(
        boundsInRoot = Rect(offset = offset, size = Size(width = viewportWidth, height = 1000f)),
        density = 1f,
      )
    }
  }
}
