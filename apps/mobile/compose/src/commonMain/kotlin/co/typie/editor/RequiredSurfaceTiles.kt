package co.typie.editor

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.unit.IntRect
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.roundToInt

internal const val EditorTileSize = 512
internal const val EditorTileGutter = 1
internal const val MaximumSurfaceTiles = 128

/** Regions are page-local logical coordinates; the grid is fixed in raster pixels. */
internal fun requiredSurfaceTiles(
  width: Double,
  height: Double,
  scaleFactor: Double,
  regions: List<Rect>?,
): List<IntRect> {
  if (!width.isFinite() || !height.isFinite() || !scaleFactor.isFinite() || scaleFactor <= 0.0)
    return emptyList()
  val pixelWidth = (width * scaleFactor).roundToInt().coerceAtLeast(1)
  val pixelHeight = (height * scaleFactor).roundToInt().coerceAtLeast(1)
  val requested = regions ?: listOf(Rect(0f, 0f, width.toFloat(), height.toFloat()))
  val tiles = mutableSetOf<IntRect>()
  for (region in requested) {
    if (
      !region.left.isFinite() ||
        !region.top.isFinite() ||
        !region.right.isFinite() ||
        !region.bottom.isFinite()
    )
      continue
    val left =
      floor(region.left.toDouble() * scaleFactor).coerceIn(0.0, pixelWidth.toDouble()).toInt()
    val top =
      floor(region.top.toDouble() * scaleFactor).coerceIn(0.0, pixelHeight.toDouble()).toInt()
    val right =
      ceil(region.right.toDouble() * scaleFactor).coerceIn(0.0, pixelWidth.toDouble()).toInt()
    val bottom =
      ceil(region.bottom.toDouble() * scaleFactor).coerceIn(0.0, pixelHeight.toDouble()).toInt()
    if (right <= left || bottom <= top) continue
    val columns = left / EditorTileSize..(right - 1) / EditorTileSize
    val rows = top / EditorTileSize..(bottom - 1) / EditorTileSize
    require(columns.count().toLong() * rows.count() <= MaximumSurfaceTiles) {
      "Visible surface exceeds tile budget"
    }
    for (row in rows) for (column in columns) {
      val x = column * EditorTileSize
      val y = row * EditorTileSize
      tiles +=
        IntRect(
          x,
          y,
          minOf(x.toLong() + EditorTileSize, pixelWidth.toLong()).toInt(),
          minOf(y.toLong() + EditorTileSize, pixelHeight.toLong()).toInt(),
        )
      require(tiles.size <= MaximumSurfaceTiles) { "Visible surface exceeds tile budget" }
    }
  }
  return tiles.sortedWith(compareBy<IntRect> { it.top }.thenBy { it.left })
}
