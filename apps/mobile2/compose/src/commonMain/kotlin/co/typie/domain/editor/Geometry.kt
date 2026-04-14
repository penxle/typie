package co.typie.domain.editor

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Size

data class PagePoint(val page: Int, val x: Float, val y: Float)

fun localToGlobal(page: Int, x: Float, y: Float, offsets: Map<Int, Offset>): Offset? {
  val offset = offsets[page] ?: return null
  return Offset(offset.x + x, offset.y + y)
}

fun globalToLocal(x: Float, y: Float, offsets: Map<Int, Offset>, sizes: List<Size>): PagePoint? {
  if (sizes.isEmpty()) return null

  var lo = 0
  var hi = sizes.lastIndex

  while (lo < hi) {
    val mid = (lo + hi) ushr 1
    val midOffset = offsets[mid] ?: return null
    if (midOffset.y + sizes[mid].height <= y) lo = mid + 1 else hi = mid
  }

  val loOffset = offsets[lo] ?: return null
  var localY = y - loOffset.y

  if (localY < 0 && lo > 0) {
    val prevOffset = offsets[lo - 1] ?: return null
    val prevBottom = prevOffset.y + sizes[lo - 1].height
    if (y < (prevBottom + loOffset.y) / 2) {
      lo--
      localY = sizes[lo].height
    } else {
      localY = 0f
    }
  }

  val finalOffset = offsets[lo] ?: return null
  val size = sizes[lo]
  val localX = (x - finalOffset.x).coerceIn(0f, size.width)
  localY = localY.coerceIn(0f, size.height)
  return PagePoint(lo, localX, localY)
}
