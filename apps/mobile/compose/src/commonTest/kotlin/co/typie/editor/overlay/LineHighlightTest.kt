package co.typie.editor.overlay

import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import kotlin.test.Test
import kotlin.test.assertEquals

class LineHighlightTest {
  private val cursor =
    CursorMetrics(
      pageIdx = 1,
      caret = Rect(x = 120f, y = 80f, width = 1f, height = 18f),
      line = Rect(x = 40f, y = 72f, width = 280f, height = 24f),
    )

  @Test
  fun `line highlight uses page wide cursor line rect in page display coordinates`() {
    val rect =
      resolveEditorLineHighlightOverlayRect(cursor = cursor, pageWidth = 360f, displayZoom = 1.5f)

    assertEquals(0f, rect.x)
    assertEquals(108f, rect.y)
    assertEquals(540f, rect.width)
    assertEquals(36f, rect.height)
  }
}
