package co.typie.editor.body

import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.scroll.EditorVisibleArea
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorBodyGeometryTest {
  @Test
  fun `geometry respects visible occlusion when resolving body height and page column width`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 720f, height = 900f),
            headerHeight = 180f,
            topInset = 120f,
            imeInset = 100f,
          ),
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = listOf(PageSize(width = 600f, height = 800f)),
        displayZoom = 1f,
      )

    assertEquals(600f, geometry.pageColumnWidth)
    assertEquals(620f, geometry.minimumBodyHeight)
    assertEquals(40f, geometry.topSpacerHeight)
  }

  @Test
  fun `geometry falls back to the visible width before page metrics arrive`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 360f, height = 640f),
            headerHeight = 72f,
            topInset = 72f,
          ),
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        pageSizes = emptyList(),
        displayZoom = 1f,
      )

    assertEquals(360f, geometry.pageColumnWidth)
    assertEquals(568f, geometry.minimumBodyHeight)
    assertEquals(40f, geometry.topSpacerHeight)
  }

  @Test
  fun `minimum body height is clamped to zero`() {
    assertEquals(
      0f,
      resolveMinimumBodyHeight(viewportHeight = 400f, headerHeight = 320f, bottomOcclusion = 120f),
    )
  }

  @Test
  fun `body fill height covers the remaining viewport below the core track`() {
    assertEquals(
      312f,
      resolveExtensionAreaFillSpacerHeight(minimumHeight = 400f, bodyContentHeight = 88f),
    )
    assertEquals(
      0f,
      resolveExtensionAreaFillSpacerHeight(minimumHeight = 400f, bodyContentHeight = 420f),
    )
  }

  @Test
  fun `geometry prefers the document layout spec over engine page widths`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 960f, height = 900f),
            headerHeight = 120f,
            topInset = 120f,
          ),
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 720f,
            pageHeight = 960f,
            pageMarginTop = 72f,
            pageMarginBottom = 72f,
            pageMarginLeft = 64f,
            pageMarginRight = 64f,
          ),
        pageSizes = listOf(PageSize(width = 700f, height = 960f)),
        displayZoom = 1f,
      )

    assertEquals(720f, geometry.pageColumnWidth)
  }

  @Test
  fun `paginated geometry scales the page column width by display zoom`() {
    val geometry =
      resolveEditorBodyGeometry(
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 960f, height = 900f),
            headerHeight = 120f,
            topInset = 120f,
          ),
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 720f,
            pageHeight = 960f,
            pageMarginTop = 72f,
            pageMarginBottom = 72f,
            pageMarginLeft = 64f,
            pageMarginRight = 64f,
          ),
        pageSizes = listOf(PageSize(width = 700f, height = 960f)),
        displayZoom = 1.25f,
      )

    assertEquals(900f, geometry.pageColumnWidth)
  }
}
