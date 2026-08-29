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
  fun `slow pinch detents at a snap point while hard bounds remain elastic`() {
    val fixture = Fixture()
    fixture.zoomController.setDisplayZoom(
      zoom = 0.95f,
      layoutSpec = fixture.layoutSpec,
      viewportWidth = fixture.viewportWidth,
    )
    fixture.uiState.updateDisplayZoom(0.95f)
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(
      fixture.semantic.updatePinch(
        start.copy(distancePx = 100f * (0.99f / 0.95f), timestampMillis = 250L)
      )
    )
    assertEquals(1f, fixture.zoomController.displayZoom, 0.0001f)

    assertTrue(
      fixture.semantic.updatePinch(
        start.copy(distancePx = 100f * (2.2f / 0.95f), timestampMillis = 500L)
      )
    )
    assertTrue(fixture.zoomController.displayZoom > 2f)
    assertTrue(fixture.zoomController.displayZoom < 2.2f)

    fixture.semantic.release()
    assertEquals(2f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(2f, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `fast pinch crosses a snap point without capture`() {
    val fixture = Fixture()
    fixture.zoomController.setDisplayZoom(
      zoom = 0.95f,
      layoutSpec = fixture.layoutSpec,
      viewportWidth = fixture.viewportWidth,
    )
    fixture.uiState.updateDisplayZoom(0.95f)
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(
      fixture.semantic.updatePinch(
        start.copy(distancePx = 100f * (0.99f / 0.95f), timestampMillis = 50L)
      )
    )

    assertEquals(0.99f, fixture.zoomController.displayZoom, 0.0001f)
  }

  @Test
  fun `release from a direct detent stays settled`() {
    val fixture = Fixture()
    fixture.zoomController.setDisplayZoom(
      zoom = 0.95f,
      layoutSpec = fixture.layoutSpec,
      viewportWidth = fixture.viewportWidth,
    )
    fixture.uiState.updateDisplayZoom(0.95f)
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(
      fixture.semantic.updatePinch(
        start.copy(distancePx = 100f * (0.99f / 0.95f), timestampMillis = 250L)
      )
    )
    fixture.semantic.release()

    assertEquals(1f, fixture.zoomController.displayZoom, 0.0001f)
  }

  @Test
  fun `fast in-range pinch stops at the released zoom`() {
    val fixture = Fixture()
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 120f, timestampMillis = 16L)))
    fixture.semantic.release()

    assertEquals(1.2f, fixture.zoomController.displayZoom, 0.0001f)
  }

  @Test
  fun `overshoot recovery sends one zoom snap haptic`() {
    val fixture = Fixture()
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 220f, timestampMillis = 250L)))
    assertTrue(fixture.zoomController.displayZoom > 2f)
    assertEquals(0, fixture.zoomSnapCount)

    fixture.semantic.release()

    assertEquals(2f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(1, fixture.zoomSnapCount)
  }

  @Test
  fun `tiny overshoot recovers to the exact bound with one haptic`() {
    val fixture = Fixture()
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(
      fixture.semantic.updatePinch(start.copy(distancePx = 200.1f, timestampMillis = 250L))
    )
    assertTrue(fixture.zoomController.displayZoom > 2f)

    fixture.semantic.release()

    assertEquals(2f, fixture.zoomController.displayZoom)
    assertEquals(1, fixture.zoomSnapCount)
  }

  @Test
  fun `first indirect update does not assume a slow velocity`() {
    val fixture = Fixture()
    fixture.zoomController.setDisplayZoom(
      zoom = 0.95f,
      layoutSpec = fixture.layoutSpec,
      viewportWidth = fixture.viewportWidth,
    )
    fixture.uiState.updateDisplayZoom(0.95f)

    assertTrue(fixture.semantic.beginIndirect())
    assertTrue(
      fixture.semantic.updateIndirectScale(
        focalInRootPx = Offset(80f, 150f),
        scaleFactor = 0.99f / 0.95f,
      )
    )

    assertEquals(0.99f, fixture.zoomController.displayZoom, 0.0001f)
  }

  @Test
  fun `overzoom stays elastic when the fit detent equals the hard bound`() {
    val fixture =
      Fixture(
        viewportWidth = 1000f,
        documentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 400f),
      )
    fixture.zoomController.setDisplayZoom(
      zoom = 2f,
      layoutSpec = fixture.layoutSpec,
      viewportWidth = fixture.viewportWidth,
    )
    fixture.uiState.updateDisplayZoom(2f)
    val start =
      EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f, timestampMillis = 0L)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 110f, timestampMillis = 250L)))

    assertTrue(fixture.zoomController.displayZoom > 2f)
    assertTrue(fixture.zoomController.displayZoom < 2.2f)
    assertEquals(0, fixture.zoomSnapCount)

    fixture.semantic.release()

    assertEquals(2f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(1, fixture.zoomSnapCount)
  }

  @Test
  fun `direct pan interruption ends transform while recovering overzoom`() {
    val fixture = Fixture()
    val start = EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 220f)))
    fixture.semantic.interruptForDirectPan()

    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(2f, fixture.zoomController.displayZoom, 0.0001f)
  }

  @Test
  fun `cancelling indirect zoom before its first sample does not strand overzoom`() {
    val fixture = Fixture()
    fixture.zoomController.setGestureDisplayZoom(
      rawZoom = 2.2f,
      layoutSpec = fixture.layoutSpec,
      viewportWidth = fixture.viewportWidth,
    )
    assertTrue(fixture.zoomController.displayZoom > 2f)

    assertTrue(fixture.semantic.beginIndirect())
    fixture.semantic.cancel()

    assertEquals(2f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(false, fixture.viewportState.isTransforming)
  }

  @Test
  fun `pinch zoom keeps the anchor under the focal point`() {
    val fixture = Fixture()

    val startSample = EditorPinchSample(focalInRootPx = Offset(80f, 150f), distancePx = 100f)
    assertTrue(fixture.semantic.beginPinch(startSample))
    assertTrue(fixture.viewportState.isTransforming)
    assertTrue(fixture.semantic.updatePinch(startSample.copy(distancePx = 150f)))

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.release()
    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `continuous pinch shares optical update anchor attachment and gesture-end commit`() {
    val fixture =
      Fixture(
        documentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = listOf(PageSize(width = 500f, height = 960f)),
        viewportWidth = 500f,
      )
    val start = EditorPinchSample(focalInRootPx = Offset(100f, 200f), distancePx = 100f)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 75f)))

    assertEquals(0.75f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(0.75f, fixture.zoomController.renderZoom, 0.0001f)
    assertTrue(fixture.attachedAnchors.isNotEmpty())

    fixture.semantic.release()
    assertEquals(0.75f, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `continuous pinch rebases its page local anchor after reflow is published`() {
    val fixture =
      Fixture(
        documentLayoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = listOf(PageSize(width = 500f, height = 960f)),
        viewportWidth = 500f,
      )
    val start = EditorPinchSample(focalInRootPx = Offset(100f, 200f), distancePx = 100f)

    assertTrue(fixture.semantic.beginPinch(start))
    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 75f)))
    fixture.updatePresentation(
      pageSizes = listOf(PageSize(width = 600f, height = 960f)),
      pageOffsets = mapOf(0 to Offset(x = 40f, y = 0f)),
      displayZoom = 0.75f,
    )

    assertTrue(fixture.semantic.updatePinch(start.copy(distancePx = 60f)))

    assertEquals(0.6f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(80f, fixture.attachedAnchors.last().first.x, 0.0001f)
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

    fixture.semantic.release()
    fixture.viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 100f, height = 120f),
      contentSize = Size(width = 500f, height = 500f),
    )

    assertEquals(Offset(x = 100f, y = 80f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `indirect scroll zoom shares the viewport zoom semantic`() {
    val fixture = Fixture()
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginIndirect())
    assertTrue(fixture.viewportState.isTransforming)
    assertTrue(
      fixture.semantic.updateIndirectScroll(
        focalInRootPx = Offset(80f, 150f),
        normalizedDelta = normalizedDeltaForOneAndHalfZoom,
      )
    )

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.release()
    assertEquals(false, fixture.viewportState.isTransforming)
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  @Test
  fun `indirect scroll anchor follows the actual page width inside the layout track`() {
    val fixture =
      Fixture(
        pageSizes = listOf(PageSize(width = 700f, height = 960f)),
        viewportWidth = 960f,
        measuredViewportSize = Size(width = 960f, height = 900f),
        contentSize = Size(width = 1080f, height = 2000f),
        pageOffsets = mapOf(0 to Offset(10f, 0f)),
      )
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginIndirect())
    assertTrue(
      fixture.semantic.updateIndirectScroll(Offset(310f, 0f), normalizedDeltaForOneAndHalfZoom)
    )

    assertEquals(Offset(35f, 0f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `indirect scroll target is restored after zoom bounds are measured`() {
    val fixture =
      Fixture(
        measuredViewportSize = Size(width = 720f, height = 900f),
        contentSize = Size(width = 720f, height = 2000f),
      )
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginIndirect())
    assertTrue(
      fixture.semantic.updateIndirectScroll(Offset(300f, 0f), normalizedDeltaForOneAndHalfZoom)
    )
    assertEquals(Offset.Zero, fixture.viewportState.scrollOffset)

    fixture.viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 720f, height = 900f),
      contentSize = Size(width = 1080f, height = 2000f),
    )

    assertEquals(Offset(150f, 0f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `indirect scroll updates remain cumulative before zoom bounds are measured`() {
    val fixture =
      Fixture(
        measuredViewportSize = Size(width = 720f, height = 900f),
        contentSize = Size(width = 720f, height = 2000f),
      )
    val normalizedDeltaForOneAndHalfZoom = -240f * ln(1.5f)

    assertTrue(fixture.semantic.beginIndirect())
    assertTrue(
      fixture.semantic.updateIndirectScroll(Offset(300f, 0f), normalizedDeltaForOneAndHalfZoom)
    )
    assertTrue(
      fixture.semantic.updateIndirectScroll(Offset(300f, 0f), normalizedDeltaForOneAndHalfZoom)
    )
    val overzoom = fixture.zoomController.displayZoom
    assertTrue(overzoom > 2f)
    assertTrue(overzoom < 2.25f)
    assertEquals(Offset.Zero, fixture.viewportState.scrollOffset)

    fixture.viewportState.updateMeasuredBounds(
      viewportSize = Size(width = 720f, height = 900f),
      contentSize = Size(width = 1440f, height = 2000f),
    )

    assertEquals(Offset(300f * (overzoom - 1f), 0f), fixture.viewportState.scrollOffset)
  }

  @Test
  fun `native scale uses the same root physical focal anchor`() {
    val fixture =
      Fixture(editorBoundsInRoot = Rect(left = 100f, top = 200f, right = 820f, bottom = 2200f))

    assertTrue(fixture.semantic.beginIndirect())
    assertTrue(
      fixture.semantic.updateIndirectScale(focalInRootPx = Offset(180f, 350f), scaleFactor = 1.5f)
    )

    assertEquals(1.5f, fixture.zoomController.displayZoom, 0.0001f)
    assertEquals(Offset(40f, 75f), fixture.viewportState.scrollOffset)

    fixture.semantic.release()
    assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom, 0.0001f)
  }

  private class Fixture(
    val pageSizes: List<PageSize> = listOf(PageSize(width = 720f, height = 960f)),
    val viewportWidth: Float = 720f,
    initialScrollOffset: Offset = Offset.Zero,
    pageOffsets: Map<Int, Offset> = mapOf(0 to Offset.Zero),
    editorBoundsInRoot: Rect = Rect(left = 0f, top = 0f, right = 720f, bottom = 2000f),
    measuredViewportSize: Size = Size(width = 100f, height = 120f),
    contentSize: Size = Size(width = 2000f, height = 2000f),
    documentLayoutSpec: EditorDocumentLayoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      ),
  ) {
    val layoutSpec = documentLayoutSpec
    val attachedAnchors =
      mutableListOf<Triple<co.typie.editor.EditorViewportAnchor, Offset, Offset>>()
    var zoomSnapCount = 0
      private set

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
    val semantic = EditorViewportZoomSemantic()

    init {
      zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = viewportWidth)
      configure(pageSizes)
    }

    fun updateEditorRootOffset(offset: Offset) {
      uiState.updateEditorBounds(
        boundsInRoot = Rect(offset = offset, size = Size(width = viewportWidth, height = 1000f)),
        density = 1f,
      )
    }

    fun updatePresentation(
      pageSizes: List<PageSize>,
      pageOffsets: Map<Int, Offset>,
      displayZoom: Float,
    ) {
      uiState.updateDisplayZoom(displayZoom)
      pageOffsets.forEach { (page, offset) ->
        uiState.updatePageOffset(page = page, offset = offset)
      }
      configure(pageSizes)
    }

    private fun configure(pageSizes: List<PageSize>) {
      semantic.configure(
        EditorViewportZoomSemanticConfig(
          layoutSpec = layoutSpec,
          zoomController = zoomController,
          viewportState = viewportState,
          uiState = uiState,
          pageSizes = pageSizes,
          viewportWidth = viewportWidth,
          density = 1f,
          onZoomSnap = { zoomSnapCount += 1 },
          onAttachViewportAnchor = { anchor, displayPosition, scrollOffset ->
            attachedAnchors += Triple(anchor, displayPosition, scrollOffset)
          },
        )
      )
    }
  }
}
