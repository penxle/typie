package co.typie.screen.editor.editor.placeholder

import androidx.compose.ui.Alignment
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.text.style.TextAlign
import co.typie.editor.body.EditorBodyGeometry
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Alignment as FfiAlignment
import co.typie.editor.ffi.PlaceholderMetrics
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.Size as PageSize
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorDocumentPlaceholderPlacementTest {
  @Test
  fun null_style_field_hides_placeholder() {
    val placement =
      resolveEditorDocumentPlaceholderPlacement(
        placeholder = samplePlaceholder(fontSize = null),
        geometry = geometry(),
        layoutSpec = continuousLayout(),
        pageSizes = listOf(PageSize(width = 600f, height = 800f)),
      )

    assertNull(placement)
  }

  @Test
  fun invalid_page_index_hides_placeholder() {
    val placement =
      resolveEditorDocumentPlaceholderPlacement(
        placeholder = samplePlaceholder(pageIdx = 1),
        geometry = geometry(),
        layoutSpec = continuousLayout(),
        pageSizes = listOf(PageSize(width = 600f, height = 800f)),
      )

    assertNull(placement)
  }

  @Test
  fun zoom_scales_rect_and_text_metrics() {
    val placement =
      resolveEditorDocumentPlaceholderPlacement(
        placeholder =
          samplePlaceholder(
            rect = Rect(x = 10f, y = 20f, width = 100f, height = 24f),
            fontSize = 1200,
            lineHeight = 160,
            letterSpacing = 5,
          ),
        geometry = geometry(pageColumnWidth = 750f, visibleBodyWidth = 360f, topSpacerHeight = 0f),
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 600f,
            pageHeight = 800f,
            pageMarginTop = 0f,
            pageMarginBottom = 0f,
            pageMarginLeft = 0f,
            pageMarginRight = 0f,
          ),
        pageSizes = listOf(PageSize(width = 600f, height = 800f)),
        displayZoom = 1.25f,
      )!!

    assertEquals(12.5f, placement.left, 0.0001f)
    assertEquals(25f, placement.top, 0.0001f)
    assertEquals(125f, placement.width, 0.0001f)
    assertEquals(20f, placement.fontSizePx, 0.0001f)
    assertEquals(1.6f, placement.lineHeightRatio, 0.0001f)
    assertEquals(0.05f, placement.letterSpacingEm, 0.0001f)
  }

  @Test
  fun center_alignment_centers_text_and_accounts_for_centered_page_column() {
    val placement =
      resolveEditorDocumentPlaceholderPlacement(
        placeholder =
          samplePlaceholder(
            rect = Rect(x = 10f, y = 20f, width = 100f, height = 24f),
            align = FfiAlignment.Center,
          ),
        geometry = geometry(pageColumnWidth = 600f, visibleBodyWidth = 800f, topSpacerHeight = 40f),
        layoutSpec = continuousLayout(),
        pageSizes = listOf(PageSize(width = 600f, height = 800f)),
      )!!

    assertEquals(110f, placement.left, 0.0001f)
    assertEquals(60f, placement.top, 0.0001f)
    assertEquals(TextAlign.Center, placement.textAlign)
    assertEquals(Alignment.CenterHorizontally, placement.horizontalAlignment)
  }

  private fun samplePlaceholder(
    pageIdx: Int = 0,
    rect: Rect = Rect(x = 0f, y = 0f, width = 100f, height = 24f),
    fontSize: Int? = 1200,
    lineHeight: Int? = 160,
    letterSpacing: Int? = 0,
    align: FfiAlignment? = FfiAlignment.Left,
  ): PlaceholderMetrics =
    PlaceholderMetrics(
      pageIdx = pageIdx,
      rect = rect,
      fontSize = fontSize,
      lineHeight = lineHeight,
      letterSpacing = letterSpacing,
      align = align,
    )

  private fun geometry(
    pageColumnWidth: Float = 600f,
    visibleBodyWidth: Float = 600f,
    topSpacerHeight: Float = 40f,
  ): EditorBodyGeometry =
    EditorBodyGeometry(
      pageColumnWidth = pageColumnWidth,
      visibleBodySize = Size(width = visibleBodyWidth, height = 640f),
      minimumBodyHeight = 640f,
      topSpacerHeight = topSpacerHeight,
    )

  private fun continuousLayout(): EditorDocumentLayoutSpec =
    EditorDocumentLayoutSpec.Continuous(maxWidth = 600f)
}
