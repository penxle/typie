package co.typie.editor

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.IntRect
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class RequiredSurfaceTilesTest {
  @Test
  fun zoomKeepsTheRasterAreaBoundedByTheViewport() {
    val density = 3.1875
    val counts =
      listOf(0.5, 1.0, 2.0).map { zoom ->
        val region =
          Rect(0f, (1200 / zoom).toFloat(), (510 / zoom).toFloat(), (2700 / zoom).toFloat())
        requiredSurfaceTiles(10_000.0, 200_000.0, density * zoom, listOf(region)).size
      }
    assertEquals(1, counts.distinct().size)
    assertTrue(counts.all { it < 48 })
  }

  @Test
  fun scrollingAndRevealPreparationShareOverlappingTiles() {
    val current = Rect(100f, 500f, 700f, 1000f)
    val target = Rect(100f, 900f, 700f, 1400f)
    val first = requiredSurfaceTiles(800.0, 200_000.0, 1.0, listOf(current))
    val prepared = requiredSurfaceTiles(800.0, 200_000.0, 1.0, listOf(current, target, current))
    assertTrue(prepared.containsAll(first))
    assertEquals(prepared.size, prepared.distinct().size)
    assertEquals(6, prepared.size)
    assertEquals(
      listOf(IntRect(512, 512, 800, 600)),
      requiredSurfaceTiles(800.0, 600.0, 1.0, listOf(Rect(512f, 512f, 999f, 999f))),
    )
    // Rust rounds a positive half pixel up; include the final one-pixel tile.
    assertEquals(
      listOf(IntRect(0, 0, 512, 1), IntRect(512, 0, 513, 1)),
      requiredSurfaceTiles(512.5, 1.0, 1.0, null),
    )
    assertFailsWith<IllegalArgumentException> {
      requiredSurfaceTiles(100_000.0, 200_000.0, 2.0, null)
    }
  }
}
