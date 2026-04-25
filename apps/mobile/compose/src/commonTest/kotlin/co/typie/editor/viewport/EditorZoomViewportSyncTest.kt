package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size as ViewportSize
import co.typie.editor.EditorViewportAnchor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class EditorZoomViewportSyncTest {
  @Test
  fun `zoom viewport scroll offset keeps first-page anchor under focal point`() {
    val scrollOffset =
      resolveZoomViewportScrollOffset(
        layoutSpec = paginatedLayout(),
        anchor = EditorViewportAnchor(page = 0, x = 120f, y = 200f),
        focalX = 80f,
        focalY = 150f,
        displayZoom = 1.5f,
        currentHorizontalScroll = 20f,
        currentVerticalScroll = 100f,
        pageSizes = listOf(PageSize(width = 720f, height = 960f)),
      )

    assertNotNull(scrollOffset)
    assertEquals(120f, scrollOffset.horizontalScroll, 0.0001f)
    assertEquals(250f, scrollOffset.verticalScroll, 0.0001f)
  }

  @Test
  fun `zoom viewport scroll offset accumulates previous page heights and gaps`() {
    val scrollOffset =
      resolveZoomViewportScrollOffset(
        layoutSpec = paginatedLayout(),
        anchor = EditorViewportAnchor(page = 2, x = 32f, y = 48f),
        focalX = 24f,
        focalY = 40f,
        displayZoom = 1.25f,
        currentHorizontalScroll = 12f,
        currentVerticalScroll = 180f,
        pageSizes =
          listOf(
            PageSize(width = 720f, height = 800f),
            PageSize(width = 720f, height = 900f),
            PageSize(width = 720f, height = 1000f),
          ),
      )

    assertNotNull(scrollOffset)
    assertEquals(28f, scrollOffset.horizontalScroll, 0.0001f)
    assertEquals(2385f, scrollOffset.verticalScroll, 0.0001f)
  }

  @Test
  fun `zoom viewport scroll offset is null when anchor page is unavailable`() {
    val scrollOffset =
      resolveZoomViewportScrollOffset(
        layoutSpec = paginatedLayout(),
        anchor = EditorViewportAnchor(page = 1, x = 0f, y = 0f),
        focalX = 0f,
        focalY = 0f,
        displayZoom = 1f,
        currentHorizontalScroll = 0f,
        currentVerticalScroll = 0f,
        pageSizes = listOf(PageSize(width = 720f, height = 960f)),
      )

    assertEquals(null, scrollOffset)
  }

  @Test
  fun `sync viewport writes the resolved target onto viewport state`() {
    val viewportState =
      EditorViewportState().apply {
        updateMeasuredBounds(
          viewportSize = ViewportSize(width = 100f, height = 120f),
          contentSize = ViewportSize(width = 500f, height = 600f),
        )
        scrollTo(offset = Offset(x = 20f, y = 100f))
      }

    syncViewportToZoomAnchor(
      viewportState = viewportState,
      layoutSpec = paginatedLayout(),
      pageSizes = listOf(PageSize(width = 720f, height = 960f)),
      anchor = EditorViewportAnchor(page = 0, x = 120f, y = 200f),
      focalX = 80f,
      focalY = 150f,
      displayZoom = 1.5f,
    )

    assertEquals(Offset(x = 120f, y = 250f), viewportState.scrollOffset)
    assertEquals(2, viewportState.lastScrollRevision)
    assertEquals(false, viewportState.lastScrollWasAuto)
  }

  private fun paginatedLayout(): EditorDocumentLayoutSpec.Paginated =
    EditorDocumentLayoutSpec.Paginated(
      pageWidth = 720f,
      pageHeight = 960f,
      pageMarginTop = 0f,
      pageMarginBottom = 0f,
      pageMarginLeft = 0f,
      pageMarginRight = 0f,
    )
}
