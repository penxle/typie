package co.typie.editor

import co.typie.editor.body.EditorDocumentLayoutSpec
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorZoomControllerTest {
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
      zoom = 1.4f,
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

    assertEquals(1.4f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)

    advanceTimeBy(119)
    runCurrent()
    assertEquals(1f, state.renderZoom, 0.0001f)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(1.4f, state.renderZoom, 0.0001f)
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
    state.setDisplayZoom(zoom = 1.4f, layoutSpec = layoutSpec, viewportWidth = 720f)

    assertEquals(1.4f, state.displayZoom, 0.0001f)
    assertEquals(1f, state.renderZoom, 0.0001f)

    state.commitRenderZoom()

    assertEquals(1.4f, state.renderZoom, 0.0001f)
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
