package co.typie.screen.editor.editor

import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.screen.editor.editor.header.resolveEditorHeaderTrackWidth
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorHeaderTrackWidthTest {
  @Test
  fun `continuous header uses the responsive fallback before page metrics arrive`() {
    val width =
      resolveEditorHeaderTrackWidth(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        resolvedPageWidth = null,
        visibleBodyWidth = 720f,
        bodyTrackWidth = 720f,
      )

    assertEquals(0f, width)
  }

  @Test
  fun `continuous header uses the resolved body track after page metrics arrive`() {
    val width =
      resolveEditorHeaderTrackWidth(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        resolvedPageWidth = 640f,
        visibleBodyWidth = 720f,
        bodyTrackWidth = 640f,
      )

    assertEquals(640f, width)
  }

  @Test
  fun `continuous header keeps the responsive fallback for an invalid page width`() {
    val width =
      resolveEditorHeaderTrackWidth(
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
        resolvedPageWidth = Float.POSITIVE_INFINITY,
        visibleBodyWidth = 720f,
        bodyTrackWidth = 720f,
      )

    assertEquals(0f, width)
  }

  @Test
  fun `paginated header follows the body page track when the viewport is wider`() {
    val width =
      resolveEditorHeaderTrackWidth(
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 720f,
            pageHeight = 960f,
            pageMarginTop = 72f,
            pageMarginBottom = 72f,
            pageMarginLeft = 64f,
            pageMarginRight = 64f,
          ),
        resolvedPageWidth = null,
        visibleBodyWidth = 960f,
        bodyTrackWidth = 720f,
      )

    assertEquals(720f, width)
  }

  @Test
  fun `paginated header preserves the web minimum content width when zoomed out`() {
    val width =
      resolveEditorHeaderTrackWidth(
        layoutSpec =
          EditorDocumentLayoutSpec.Paginated(
            pageWidth = 720f,
            pageHeight = 960f,
            pageMarginTop = 72f,
            pageMarginBottom = 72f,
            pageMarginLeft = 64f,
            pageMarginRight = 64f,
          ),
        resolvedPageWidth = null,
        visibleBodyWidth = 320f,
        bodyTrackWidth = 180f,
      )

    assertEquals(272f, width)
  }
}
