package co.typie.editor

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveBaseBottomSpace
import co.typie.editor.body.resolveContinuousLayoutViewportWidth
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorZoomControllerTest {
  @Test
  fun `continuous policy mirrors web bounds fit snap and logical viewport`() {
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)

    assertEquals(0.15625f, computeDocumentZoomBounds(layout).start, 0.0001f)
    assertEquals(2f, computeDocumentZoomBounds(layout).endInclusive, 0.0001f)
    assertEquals(0.78125f, computeDocumentFitWidthZoom(layout, 500f), 0.0001f)
    assertEquals(0.78125f, clampDocumentLayoutZoom(0.79f, layout, 500f), 0.0001f)
    assertEquals(500f, resolveContinuousLayoutViewportWidth(500f, 1.5f), 0.0001f)
    assertEquals(625f, resolveContinuousLayoutViewportWidth(500f, 0.8f), 0.0001f)
  }

  @Test
  fun `continuous engine margin follows visual zoom`() {
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)

    assertEquals(30f, layout.resolveBaseBottomSpace(displayZoom = 1.5f), 0.0001f)
    assertEquals(10f, layout.resolveBaseBottomSpace(displayZoom = 0.5f), 0.0001f)
  }

  @Test
  fun `paginated sync applies fit-width initial zoom`() {
    val state = EditorZoomController()

    state.syncLayout(
      layoutSpec =
        EditorDocumentLayoutSpec.Paginated(
          pageWidth = 720f,
          pageHeight = 960f,
          pageMarginTop = 72f,
          pageMarginBottom = 72f,
          pageMarginLeft = 64f,
          pageMarginRight = 64f,
        ),
      viewportWidth = 360f,
    )

    assertEquals(0.5f, state.displayZoom, 0.0001f)
    assertEquals(0.5f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `continuous sync resets zoom to unit`() {
    val state = EditorZoomController()
    state.setDisplayZoom(
      zoom = 1.6f,
      layoutSpec =
        EditorDocumentLayoutSpec.Paginated(
          pageWidth = 720f,
          pageHeight = 960f,
          pageMarginTop = 72f,
          pageMarginBottom = 72f,
          pageMarginLeft = 64f,
          pageMarginRight = 64f,
        ),
      viewportWidth = 960f,
    )

    state.syncLayout(
      layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
      viewportWidth = 960f,
    )

    assertEquals(1f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `layout sync does not resnap a settled zoom when the viewport resizes`() {
    val state = EditorZoomController()
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)
    state.setDisplayZoom(zoom = 0.79f, layoutSpec = layout, viewportWidth = 500f)
    assertEquals(0.78125f, state.displayZoom, 0.0001f)

    state.syncLayout(layoutSpec = layout, viewportWidth = 496f)

    assertEquals(0.78125f, state.displayZoom, 0.0001f)
    assertNull(state.resolveLandmark())
  }

  @Test
  fun `paginated zoom is clamped and snaps near unit zoom`() {
    val state = EditorZoomController()

    state.setDisplayZoom(
      zoom = 0.99f,
      layoutSpec =
        EditorDocumentLayoutSpec.Paginated(
          pageWidth = 720f,
          pageHeight = 960f,
          pageMarginTop = 72f,
          pageMarginBottom = 72f,
          pageMarginLeft = 64f,
          pageMarginRight = 64f,
        ),
      viewportWidth = 720f,
    )

    assertEquals(1f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `direct gesture zoom rubber bands beyond bounds while programmatic zoom stays clamped`() {
    val state = EditorZoomController()
    val layout =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 72f,
        pageMarginBottom = 72f,
        pageMarginLeft = 64f,
        pageMarginRight = 64f,
      )
    state.syncLayout(layoutSpec = layout, viewportWidth = 720f)

    state.setGestureDisplayZoom(rawZoom = 2.2f, layoutSpec = layout, viewportWidth = 720f)
    assertTrue(state.displayZoom > 2f)
    assertTrue(state.displayZoom < 2.2f)
    assertEquals(2f, state.indicatorZoom, 0.0001f)

    state.setDisplayZoom(zoom = state.displayZoom, layoutSpec = layout, viewportWidth = 720f)
    assertEquals(2f, state.displayZoom, 0.0001f)
  }

  @Test
  fun `render zoom follows display zoom after debounce`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)

    state.syncLayout(
      layoutSpec =
        EditorDocumentLayoutSpec.Paginated(
          pageWidth = 720f,
          pageHeight = 960f,
          pageMarginTop = 72f,
          pageMarginBottom = 72f,
          pageMarginLeft = 64f,
          pageMarginRight = 64f,
        ),
      viewportWidth = 720f,
    )

    state.setDisplayZoom(
      zoom = 1.1f,
      layoutSpec =
        EditorDocumentLayoutSpec.Paginated(
          pageWidth = 720f,
          pageHeight = 960f,
          pageMarginTop = 72f,
          pageMarginBottom = 72f,
          pageMarginLeft = 64f,
          pageMarginRight = 64f,
        ),
      viewportWidth = 720f,
    )

    assertEquals(1.1f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)

    advanceTimeBy(119)
    runCurrent()
    assertEquals(1f, state.renderZoom, 0.0001f)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(1.1f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `continuous render zoom follows display zoom at the same debounce boundary`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    state.setDisplayZoom(zoom = 0.9f, layoutSpec = layout, viewportWidth = 500f)

    assertEquals(0.9f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)
    advanceTimeBy(119)
    runCurrent()
    assertEquals(1f, state.renderZoom, 0.0001f)
    advanceTimeBy(1)
    runCurrent()
    assertEquals(0.9f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `commit render zoom syncs render zoom immediately`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 72f,
        pageMarginBottom = 72f,
        pageMarginLeft = 64f,
        pageMarginRight = 64f,
      )

    state.syncLayout(layoutSpec = layoutSpec, viewportWidth = 720f)
    state.setDisplayZoom(zoom = 1.1f, layoutSpec = layoutSpec, viewportWidth = 720f)

    assertEquals(1.1f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)

    state.commitRenderZoom()

    assertEquals(1.1f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `large scale difference commits without waiting for the quiet period`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    state.setDisplayZoom(zoom = 0.84f, layoutSpec = layout, viewportWidth = 500f)

    assertEquals(0.84f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `rapid large scale changes keep the minimum render commit interval`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    state.setDisplayZoom(zoom = 0.84f, layoutSpec = layout, viewportWidth = 500f)
    state.setDisplayZoom(zoom = 1.2f, layoutSpec = layout, viewportWidth = 500f)
    advanceTimeBy(159)
    state.setDisplayZoom(zoom = 1.3f, layoutSpec = layout, viewportWidth = 500f)

    assertEquals(0.84f, state.renderZoom, 0.0001f)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(1.3f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `latest small change replaces a blocked threshold commit without extending maximum delay`() =
    runTest {
      val state = EditorZoomController(scope = backgroundScope)
      val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
      state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

      state.setDisplayZoom(zoom = 0.84f, layoutSpec = layout, viewportWidth = 500f)
      advanceTimeBy(20)
      state.setDisplayZoom(zoom = 1.2f, layoutSpec = layout, viewportWidth = 500f)
      advanceTimeBy(80)
      state.setDisplayZoom(zoom = 0.9f, layoutSpec = layout, viewportWidth = 500f)
      advanceTimeBy(100)
      state.setDisplayZoom(zoom = 0.91f, layoutSpec = layout, viewportWidth = 500f)
      advanceTimeBy(100)
      state.setDisplayZoom(zoom = 0.9f, layoutSpec = layout, viewportWidth = 500f)
      advanceTimeBy(19)
      runCurrent()

      assertEquals(0.84f, state.renderZoom, 0.0001f)

      advanceTimeBy(1)
      runCurrent()
      assertEquals(0.9f, state.renderZoom, 0.0001f)
    }

  @Test
  fun `continuous input commits by the maximum render delay`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    state.setDisplayZoom(zoom = 0.95f, layoutSpec = layout, viewportWidth = 500f)
    advanceTimeBy(100)
    state.setDisplayZoom(zoom = 0.96f, layoutSpec = layout, viewportWidth = 500f)
    advanceTimeBy(100)
    state.setDisplayZoom(zoom = 0.95f, layoutSpec = layout, viewportWidth = 500f)
    advanceTimeBy(99)
    runCurrent()

    assertEquals(1f, state.renderZoom, 0.0001f)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(0.95f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `gesture end waits for the minimum interval after an intermediate commit`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layout = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    state.setDisplayZoom(zoom = 0.84f, layoutSpec = layout, viewportWidth = 500f)
    state.setDisplayZoom(zoom = 1.1f, layoutSpec = layout, viewportWidth = 500f)
    state.commitRenderZoom()

    assertEquals(0.84f, state.renderZoom, 0.0001f)

    advanceTimeBy(160)
    runCurrent()
    assertEquals(1.1f, state.renderZoom, 0.0001f)
  }

  @Test
  fun `controller exposes zoom and snap key for paginated layout`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 72f,
        pageMarginBottom = 72f,
        pageMarginLeft = 64f,
        pageMarginRight = 64f,
      )

    state.syncLayout(layoutSpec = layoutSpec, viewportWidth = 720f)

    assertEquals(1f, state.displayZoom, 0.0001f)
    assertEquals(EditorZoomSnapKey.FitWidth, state.resolveSnapKey())
  }

  @Test
  fun `controller resolves fit-width snap key`() = runTest {
    val state = EditorZoomController(scope = backgroundScope)
    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 72f,
        pageMarginBottom = 72f,
        pageMarginLeft = 64f,
        pageMarginRight = 64f,
      )

    state.syncLayout(layoutSpec = layoutSpec, viewportWidth = 360f)

    assertEquals(0.5f, state.displayZoom, 0.0001f)
    assertEquals(EditorZoomSnapKey.FitWidth, state.resolveSnapKey())
  }

  @Test
  fun `landmark resolver names bounds fit unit and unnamed zooms`() {
    val layout = paginatedLayout(pageWidth = 1000f)

    assertEquals(EditorZoomLandmark.Minimum, resolveEditorZoomLandmark(0.1f, layout, 500f))
    assertEquals(EditorZoomLandmark.FitWidth, resolveEditorZoomLandmark(0.5f, layout, 500f))
    assertNull(resolveEditorZoomLandmark(0.75f, layout, 500f))
    assertEquals(EditorZoomLandmark.Unit, resolveEditorZoomLandmark(1f, layout, 500f))
    assertEquals(EditorZoomLandmark.Maximum, resolveEditorZoomLandmark(2f, layout, 500f))
  }

  @Test
  fun `landmark resolver prefers unit then fit-width over bounds`() {
    val layout = paginatedLayout(pageWidth = 1000f)

    assertEquals(EditorZoomLandmark.Unit, resolveEditorZoomLandmark(1f, layout, 1000f))
    assertEquals(EditorZoomLandmark.FitWidth, resolveEditorZoomLandmark(0.1f, layout, 100f))
  }

  @Test
  fun `landmark resolver does not name clamped fit targets as fit-width`() {
    val layout = paginatedLayout(pageWidth = 1000f)

    assertEquals(EditorZoomLandmark.Minimum, resolveEditorZoomLandmark(0.1f, layout, 50f))
    assertEquals(EditorZoomLandmark.Maximum, resolveEditorZoomLandmark(2f, layout, 2500f))
  }

  @Test
  fun `landmark resolver rejects invalid input`() {
    val layout = paginatedLayout(pageWidth = 1000f)

    assertNull(resolveEditorZoomLandmark(Float.NaN, layout, 500f))
    assertNull(resolveEditorZoomLandmark(1f, layout, 0f))
    assertNull(resolveEditorZoomLandmark(1f, layout, Float.NaN))
    assertNull(resolveEditorZoomLandmark(1f, paginatedLayout(Float.NaN), 500f))
  }

  @Test
  fun `indicator toggle target switches unit and fit and returns other zooms to unit`() {
    val state = EditorZoomController()
    val layout = paginatedLayout(pageWidth = 1000f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    assertEquals(1f, state.resolveIndicatorToggleTarget())
    state.setDisplayZoom(zoom = 1f, layoutSpec = layout, viewportWidth = 500f)
    assertEquals(0.5f, state.resolveIndicatorToggleTarget())
    state.setDisplayZoom(zoom = 1.4f, layoutSpec = layout, viewportWidth = 500f)
    assertEquals(1f, state.resolveIndicatorToggleTarget())
  }

  @Test
  fun `indicator toggle has no target when unit equals fit-width`() {
    val state = EditorZoomController()
    val layout = paginatedLayout(pageWidth = 1000f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 1000f)

    assertNull(state.resolveIndicatorToggleTarget())
  }

  @Test
  fun `button step targets stop at the normal zoom bounds`() {
    val state = EditorZoomController()
    val layout = paginatedLayout(pageWidth = 1000f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    assertEquals(0.6f, requireNotNull(state.resolveZoomInTarget()), 0.0001f)
    assertEquals(0.4f, requireNotNull(state.resolveZoomOutTarget()), 0.0001f)
    state.setDisplayZoom(zoom = 2f, layoutSpec = layout, viewportWidth = 500f)
    assertNull(state.resolveZoomInTarget())
    state.setDisplayZoom(zoom = 0.1f, layoutSpec = layout, viewportWidth = 500f)
    assertNull(state.resolveZoomOutTarget())
  }

  @Test
  fun `button steps move to the next ten-percent grid line`() {
    val state = EditorZoomController()
    val layout = paginatedLayout(pageWidth = 1000f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 1000f)

    state.setDisplayZoom(zoom = 0.53f, layoutSpec = layout, viewportWidth = 1000f)
    assertEquals(0.6f, requireNotNull(state.resolveZoomInTarget()), 0.0001f)
    assertEquals(0.5f, requireNotNull(state.resolveZoomOutTarget()), 0.0001f)
  }

  @Test
  fun `button steps stop at landmarks between ten-percent grid lines`() {
    val state = EditorZoomController()
    val layout = paginatedLayout(pageWidth = 1000f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 520f)

    state.setDisplayZoom(zoom = 0.45f, layoutSpec = layout, viewportWidth = 520f)
    assertEquals(0.5f, requireNotNull(state.resolveZoomInTarget()), 0.0001f)
    state.setDisplayZoom(
      zoom = 0.5f,
      layoutSpec = layout,
      viewportWidth = 520f,
      snapToLandmarks = false,
    )
    assertEquals(0.52f, requireNotNull(state.resolveZoomInTarget()), 0.0001f)

    state.setDisplayZoom(zoom = 0.58f, layoutSpec = layout, viewportWidth = 520f)
    assertEquals(0.52f, requireNotNull(state.resolveZoomOutTarget()), 0.0001f)
  }

  @Test
  fun `button step targets stay unavailable while elastically overshooting their bound`() {
    val state = EditorZoomController()
    val layout = paginatedLayout(pageWidth = 1000f)
    state.syncLayout(layoutSpec = layout, viewportWidth = 500f)

    state.setGestureDisplayZoom(rawZoom = 0.05f, layoutSpec = layout, viewportWidth = 500f)
    assertTrue(state.displayZoom < state.indicatorZoom)
    assertNull(state.resolveZoomOutTarget())

    state.setGestureDisplayZoom(rawZoom = 2.2f, layoutSpec = layout, viewportWidth = 500f)
    assertTrue(state.displayZoom > state.indicatorZoom)
    assertNull(state.resolveZoomInTarget())
  }
}

private fun paginatedLayout(pageWidth: Float) =
  EditorDocumentLayoutSpec.Paginated(
    pageWidth = pageWidth,
    pageHeight = 1200f,
    pageMarginTop = 72f,
    pageMarginBottom = 72f,
    pageMarginLeft = 64f,
    pageMarginRight = 64f,
  )
