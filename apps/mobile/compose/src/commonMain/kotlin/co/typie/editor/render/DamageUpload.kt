package co.typie.editor.render

internal data class RowRange(val minY: Int, val maxY: Int)

internal fun damageRowRange(damage: IntArray, count: Int, height: Int): RowRange {
  if (count <= 0) return RowRange(0, 0)
  var minY = Int.MAX_VALUE
  var maxY = Int.MIN_VALUE
  for (i in 0 until count) {
    val y0 = damage[i * 4 + 1]
    val y1 = damage[i * 4 + 3]
    if (y0 < minY) minY = y0
    if (y1 > maxY) maxY = y1
  }
  val clampedMinY = minY.coerceIn(0, height)
  val clampedMaxY = maxY.coerceIn(0, height)
  return if (clampedMinY < clampedMaxY) RowRange(clampedMinY, clampedMaxY) else RowRange(0, 0)
}

internal fun shouldPartialUpload(
  hasCached: Boolean,
  cachedW: Int,
  cachedH: Int,
  w: Int,
  h: Int,
  readerLastVersion: Long,
  damageFrom: Long,
  damageCount: Int,
): Boolean =
  hasCached &&
    cachedW == w &&
    cachedH == h &&
    readerLastVersion > 0L &&
    readerLastVersion >= damageFrom &&
    damageCount > 0
