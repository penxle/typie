package co.typie.editor

import androidx.compose.ui.geometry.Rect
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Size as PageSize

data class VerticalSpan(val top: Float = 0f, val bottom: Float = 0f) {
  val isValid: Boolean
    get() = bottom > top

  val height: Float
    get() = (bottom - top).coerceAtLeast(0f)
}

internal fun pageRectsToContentRect(
  rects: Iterable<PageRect>,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float = 1f,
  density: Float = 0f,
  contentOriginX: Float = 0f,
  contentOriginY: Float = 0f,
): Rect? {
  val zoom = normalizeDisplayZoom(displayZoom)
  return unionRects(
    rects.mapNotNull { pageRect ->
      val pageTop =
        layoutSpec.resolvePageContentTop(
          page = pageRect.pageIdx,
          pageSizes = pageSizes,
          displayZoom = zoom,
          density = density,
        ) ?: return@mapNotNull null
      val rect = pageRect.rect
      val left = contentOriginX + rect.x * zoom
      val top = contentOriginY + pageTop + rect.y * zoom
      Rect(
        left = left,
        top = top,
        right = left + rect.width * zoom,
        bottom = top + rect.height * zoom,
      )
    }
  )
}

internal fun unionRects(rects: Iterable<Rect?>): Rect? {
  var left = Float.POSITIVE_INFINITY
  var top = Float.POSITIVE_INFINITY
  var right = Float.NEGATIVE_INFINITY
  var bottom = Float.NEGATIVE_INFINITY

  for (rect in rects) {
    if (
      rect == null ||
        !rect.left.isFinite() ||
        !rect.top.isFinite() ||
        !rect.right.isFinite() ||
        !rect.bottom.isFinite()
    ) {
      continue
    }
    left = minOf(left, rect.left, rect.right)
    top = minOf(top, rect.top, rect.bottom)
    right = maxOf(right, rect.left, rect.right)
    bottom = maxOf(bottom, rect.top, rect.bottom)
  }

  return if (left == Float.POSITIVE_INFINITY) {
    null
  } else {
    Rect(left = left, top = top, right = right, bottom = bottom)
  }
}

private fun normalizeDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }
