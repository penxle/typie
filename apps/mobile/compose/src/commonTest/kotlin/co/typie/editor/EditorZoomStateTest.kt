package co.typie.editor

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolveBaseBottomSpace
import co.typie.editor.body.resolveContinuousLayoutViewportWidth
import kotlin.test.Test
import kotlin.test.assertEquals
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
}
