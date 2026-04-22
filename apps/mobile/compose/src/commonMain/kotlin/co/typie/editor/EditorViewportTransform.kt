package co.typie.editor

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.Size
import kotlin.math.max

data class PagePoint(val page: Int, val x: Float, val y: Float)

data class VerticalSpan(val top: Float = 0f, val bottom: Float = 0f) {
  val isValid: Boolean
    get() = bottom > top

  val height: Float
    get() = max(0f, bottom - top)
}

data class EditorViewportAnchor(val page: Int, val x: Float, val y: Float)

data class EditorViewportTransform(
  val pageOffsets: Map<Int, Offset>,
  val pageSizes: List<Size>,
  val displayZoom: Float = 1f,
) {
  private val effectiveDisplayZoom = normalizeDisplayZoom(displayZoom)

  fun localToGlobal(page: Int, x: Float, y: Float): Offset? {
    val offset = pageOffsets[page] ?: return null
    return Offset(offset.x + x * effectiveDisplayZoom, offset.y + y * effectiveDisplayZoom)
  }

  fun globalToLocal(x: Float, y: Float): PagePoint? {
    if (pageSizes.isEmpty()) return null

    var lo = 0
    var hi = pageSizes.lastIndex

    while (lo < hi) {
      val mid = (lo + hi) ushr 1
      val midOffset = pageOffsets[mid] ?: return null
      if (midOffset.y + pageSizes[mid].height * effectiveDisplayZoom <= y) lo = mid + 1
      else hi = mid
    }

    val loOffset = pageOffsets[lo] ?: return null
    var localY = y - loOffset.y

    if (localY < 0f && lo > 0) {
      val previousPage = lo - 1
      val previousOffset = pageOffsets[previousPage] ?: return null
      val previousBottom = previousOffset.y + pageSizes[previousPage].height * effectiveDisplayZoom
      if (y < (previousBottom + loOffset.y) / 2f) {
        lo = previousPage
        localY = pageSizes[lo].height * effectiveDisplayZoom
      } else {
        localY = 0f
      }
    }

    val finalOffset = pageOffsets[lo] ?: return null
    val size = pageSizes[lo]
    val localX = ((x - finalOffset.x) / effectiveDisplayZoom).coerceIn(0f, size.width)
    val clampedLocalY = (localY / effectiveDisplayZoom).coerceIn(0f, size.height)
    return PagePoint(lo, localX, clampedLocalY)
  }

  fun resolveAnchor(focalX: Float, focalY: Float): EditorViewportAnchor? {
    val point = globalToLocal(x = focalX, y = focalY) ?: return null
    return EditorViewportAnchor(page = point.page, x = point.x, y = point.y)
  }

  fun positionOf(anchor: EditorViewportAnchor): Offset? =
    localToGlobal(anchor.page, anchor.x, anchor.y)
}

fun localToGlobal(
  page: Int,
  x: Float,
  y: Float,
  pageOffsets: Map<Int, Offset>,
  displayZoom: Float = 1f,
): Offset? =
  EditorViewportTransform(
      pageOffsets = pageOffsets,
      pageSizes = emptyList(),
      displayZoom = displayZoom,
    )
    .localToGlobal(page = page, x = x, y = y)

fun globalToLocal(
  x: Float,
  y: Float,
  pageOffsets: Map<Int, Offset>,
  sizes: List<Size>,
  displayZoom: Float = 1f,
): PagePoint? =
  EditorViewportTransform(pageOffsets = pageOffsets, pageSizes = sizes, displayZoom = displayZoom)
    .globalToLocal(x = x, y = y)

private fun normalizeDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }
