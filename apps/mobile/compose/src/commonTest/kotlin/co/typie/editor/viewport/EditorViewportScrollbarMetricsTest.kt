package co.typie.editor.viewport

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse

class EditorViewportScrollbarMetricsTest {
  @Test
  fun `scrollbar is hidden when visible track cannot fit the minimum thumb`() {
    val metrics =
      resolveEditorViewportScrollbarMetrics(
        viewportLength = 900f,
        contentLength = 1800f,
        scrollPosition = 240f,
        minThumbSize = 30f,
        leadingInset = 120f,
        trailingInset = 755f,
      )

    assertFalse(metrics.isVisible)
    assertEquals(25f, metrics.trackLength)
    assertEquals(0f, metrics.thumbSize)
    assertEquals(0f, metrics.thumbOffset)
    assertEquals(25f, metrics.thumbTravel)
  }
}
