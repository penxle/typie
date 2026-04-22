package co.typie.editor

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Size
import kotlin.math.max

// `global` here means coordinates in the editor viewport, not the app window/root.
data class PagePoint(val page: Int, val x: Float, val y: Float)

data class VerticalSpan(val top: Float = 0f, val bottom: Float = 0f) {
  val isValid: Boolean
    get() = bottom > top

  val height: Float
    get() = max(0f, bottom - top)
}

fun localToGlobal(page: Int, x: Float, y: Float, pageOffsets: Map<Int, Offset>): Offset? {
  val offset = pageOffsets[page] ?: return null
  return Offset(offset.x + x, offset.y + y)
}

fun globalToLocal(
  x: Float,
  y: Float,
  pageOffsets: Map<Int, Offset>,
  sizes: List<Size>,
): PagePoint? {
  if (sizes.isEmpty()) return null

  var lo = 0
  var hi = sizes.lastIndex

  while (lo < hi) {
    val mid = (lo + hi) ushr 1
    val midOffset = pageOffsets[mid] ?: return null
    if (midOffset.y + sizes[mid].height <= y) lo = mid + 1 else hi = mid
  }

  val loOffset = pageOffsets[lo] ?: return null
  var localY = y - loOffset.y

  if (localY < 0 && lo > 0) {
    val prevOffset = pageOffsets[lo - 1] ?: return null
    val prevBottom = prevOffset.y + sizes[lo - 1].height
    if (y < (prevBottom + loOffset.y) / 2) {
      lo--
      localY = sizes[lo].height
    } else {
      localY = 0f
    }
  }

  val finalOffset = pageOffsets[lo] ?: return null
  val size = sizes[lo]
  val localX = (x - finalOffset.x).coerceIn(0f, size.width)
  localY = localY.coerceIn(0f, size.height)
  return PagePoint(lo, localX, localY)
}
