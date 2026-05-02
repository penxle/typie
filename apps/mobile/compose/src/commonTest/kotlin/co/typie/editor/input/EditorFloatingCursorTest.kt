package co.typie.editor.input

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportTransform
import co.typie.editor.ffi.Size
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorFloatingCursorTest {
  @Test
  fun `floating cursor delta is mapped through viewport zoom`() {
    val point =
      resolveFloatingCursorPoint(
        origin = EditorFloatingCursorOrigin(page = 0, x = 10f, y = 20f),
        dx = 20f,
        dy = 10f,
        transform =
          EditorViewportTransform(
            pageOffsets = mapOf(0 to Offset.Zero),
            pageSizes = listOf(Size(width = 100f, height = 100f)),
            displayZoom = 2f,
          ),
      )

    assertEquals(0, point?.page)
    assertEquals(20f, point?.x)
    assertEquals(25f, point?.y)
  }

  @Test
  fun `floating cursor can cross page boundary`() {
    val point =
      resolveFloatingCursorPoint(
        origin = EditorFloatingCursorOrigin(page = 0, x = 10f, y = 90f),
        dx = 0f,
        dy = 30f,
        transform =
          EditorViewportTransform(
            pageOffsets = mapOf(0 to Offset(0f, 0f), 1 to Offset(0f, 120f)),
            pageSizes =
              listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f)),
            displayZoom = 1f,
          ),
      )

    assertEquals(1, point?.page)
    assertEquals(10f, point?.x)
    assertEquals(0f, point?.y)
  }

  @Test
  fun `floating cursor returns null when origin page is missing`() {
    val point =
      resolveFloatingCursorPoint(
        origin = EditorFloatingCursorOrigin(page = 1, x = 10f, y = 20f),
        dx = 0f,
        dy = 0f,
        transform =
          EditorViewportTransform(
            pageOffsets = mapOf(0 to Offset.Zero),
            pageSizes = listOf(Size(width = 100f, height = 100f)),
          ),
      )

    assertNull(point)
  }
}
