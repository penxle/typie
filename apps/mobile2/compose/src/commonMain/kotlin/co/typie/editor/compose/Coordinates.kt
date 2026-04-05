package co.typie.editor.compose

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Size

// All inputs/outputs are in dp.

internal fun localToGlobal(page: Int, x: Float, y: Float, pageOffsets: Map<Int, Offset>): Offset? {
  val offset = pageOffsets[page] ?: return null
  return Offset(offset.x + x, offset.y + y)
}

internal fun globalToLocal(
  x: Float,
  y: Float,
  pageOffsets: Map<Int, Offset>,
  pageSizes: List<Size>,
): PagePoint? {
  if (pageSizes.isEmpty()) return null

  var lo = 0
  var hi = pageSizes.lastIndex

  while (lo < hi) {
    val mid = (lo + hi) ushr 1
    val midOffset = pageOffsets[mid] ?: return null
    if (midOffset.y + pageSizes[mid].height <= y) lo = mid + 1
    else hi = mid
  }

  val loOffset = pageOffsets[lo] ?: return null
  var localY = y - loOffset.y

  // Snap to nearest page when y falls in the gap between two pages
  if (localY < 0 && lo > 0) {
    val prevOffset = pageOffsets[lo - 1] ?: return null
    val prevBottom = prevOffset.y + pageSizes[lo - 1].height
    if (y < (prevBottom + loOffset.y) / 2) {
      lo--
      localY = pageSizes[lo].height
    } else {
      localY = 0f
    }
  }

  val finalOffset = pageOffsets[lo] ?: return null
  val size = pageSizes[lo]
  val localX = (x - finalOffset.x).coerceIn(0f, size.width)
  localY = localY.coerceIn(0f, size.height)
  return PagePoint(lo, localX, localY)
}
