package co.typie.editor.overlay

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import co.typie.editor.runtime.EditorBoundsInContainer
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class LineHighlightTest {
  private val cursor =
    CursorMetrics(
      pageIdx = 1,
      caret = Rect(x = 120f, y = 80f, width = 1f, height = 18f),
      line = Rect(x = 40f, y = 72f, width = 280f, height = 24f),
    )

  @Test
  fun `continuous line highlight resolves into extension area coordinates`() {
    val band =
      resolveEditorExtensionAreaLineHighlightBand(
        cursor = cursor,
        editorBounds = EditorBoundsInContainer(x = 20f, y = 40f, width = 640f, height = 900f),
        viewportTransform =
          EditorViewportTransform(
            pageOffsets = mapOf(1 to Offset(x = 0f, y = 12f)),
            displayZoom = 1.5f,
          ),
      )

    assertEquals(EditorLineHighlightBand(top = 160f, height = 36f), band)
  }

  @Test
  fun `continuous line highlight is unresolved until its page position is measured`() {
    val band =
      resolveEditorExtensionAreaLineHighlightBand(
        cursor = cursor,
        editorBounds = EditorBoundsInContainer(x = 20f, y = 40f, width = 640f, height = 900f),
        viewportTransform = EditorViewportTransform(pageOffsets = emptyMap(), displayZoom = 1.5f),
      )

    assertNull(band)
  }

  @Test
  fun `paginated line highlight stays page wide in page display coordinates`() {
    val rect =
      resolveEditorLineHighlightOverlayRect(cursor = cursor, pageWidth = 360f, displayZoom = 1.5f)

    assertEquals(0f, rect.x)
    assertEquals(108f, rect.y)
    assertEquals(540f, rect.width)
    assertEquals(36f, rect.height)
  }
}
