package co.typie.editor.overlay

import co.typie.editor.ffi.CaretMetrics
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Rect
import kotlin.test.Test
import kotlin.test.assertEquals

class CursorTest {
  @Test
  fun `cursor overlay scales position and height but keeps platform width`() {
    val rect =
      resolveEditorCursorOverlayRect(
        cursor =
          CursorMetrics(
            pageIdx = 2,
            caret = CaretMetrics(x = 120f, y = 80f, height = 18f),
            line = Rect(x = 40f, y = 72f, width = 280f, height = 24f),
          ),
        displayZoom = 1.5f,
      )

    assertEquals(180f, rect.x)
    assertEquals(120f, rect.y)
    assertEquals(2f, rect.width)
    assertEquals(27f, rect.height)
  }
}
