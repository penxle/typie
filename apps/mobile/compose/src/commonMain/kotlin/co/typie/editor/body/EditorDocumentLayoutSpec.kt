package co.typie.editor.body

import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Size as PageSize
import kotlin.math.round

internal const val PaginatedPageGap = 24f

internal sealed interface EditorDocumentLayoutSpec {
  data class Continuous(val maxWidth: Float) : EditorDocumentLayoutSpec

  data class Paginated(
    val pageWidth: Float,
    val pageHeight: Float,
    val pageMarginTop: Float,
    val pageMarginBottom: Float,
    val pageMarginLeft: Float,
    val pageMarginRight: Float,
  ) : EditorDocumentLayoutSpec
}

internal fun LayoutMode.toEditorDocumentLayoutSpec(): EditorDocumentLayoutSpec =
  when (this) {
    is LayoutMode.Continuous -> EditorDocumentLayoutSpec.Continuous(maxWidth = maxWidth.toFloat())
    is LayoutMode.Paginated ->
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = pageWidth.toFloat(),
        pageHeight = pageHeight.toFloat(),
        pageMarginTop = pageMarginTop.toFloat(),
        pageMarginBottom = pageMarginBottom.toFloat(),
        pageMarginLeft = pageMarginLeft.toFloat(),
        pageMarginRight = pageMarginRight.toFloat(),
      )
  }

internal fun EditorDocumentLayoutSpec.resolveBaseBottomSpace(displayZoom: Float = 1f): Float =
  when (this) {
    is EditorDocumentLayoutSpec.Continuous -> 20f
    is EditorDocumentLayoutSpec.Paginated -> pageMarginBottom * displayZoom
  }

internal fun resolvePaginatedPageGap(displayZoom: Float = 1f): Float =
  PaginatedPageGap * normalizeDisplayZoom(displayZoom)

internal fun EditorDocumentLayoutSpec.resolvePagesContentHeight(
  pageSizes: List<PageSize>,
  displayZoom: Float = 1f,
  density: Float = 0f,
): Float {
  val effectiveDisplayZoom = normalizeDisplayZoom(displayZoom)
  val measuredPageGap =
    resolveMeasuredPageGapLength(displayZoom = effectiveDisplayZoom, density = density)
  return pageSizes.foldIndexed(0f) { index, total, size ->
    total +
      resolveMeasuredPageLength(size.height, effectiveDisplayZoom, density) +
      if (index < pageSizes.lastIndex) measuredPageGap else 0f
  }
}

internal fun EditorDocumentLayoutSpec.resolvePageContentTop(
  page: Int,
  pageSizes: List<PageSize>,
  displayZoom: Float = 1f,
  density: Float = 0f,
): Float? {
  if (page !in pageSizes.indices) {
    return null
  }

  val effectiveDisplayZoom = normalizeDisplayZoom(displayZoom)
  val measuredPageGap =
    resolveMeasuredPageGapLength(displayZoom = effectiveDisplayZoom, density = density)
  var top = 0f
  repeat(page) { index ->
    top += resolveMeasuredPageLength(pageSizes[index].height, effectiveDisplayZoom, density)
    top += measuredPageGap
  }
  return top
}

private fun EditorDocumentLayoutSpec.resolveMeasuredPageGapLength(
  displayZoom: Float = 1f,
  density: Float = 0f,
): Float {
  val effectiveDisplayZoom = normalizeDisplayZoom(displayZoom)
  val pageGap =
    when (this) {
      is EditorDocumentLayoutSpec.Paginated -> resolvePaginatedPageGap(effectiveDisplayZoom)
      is EditorDocumentLayoutSpec.Continuous -> 0f
    }
  return pageGap.resolveMeasuredLength(density = density, minimumPx = 0f)
}

private fun normalizeDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }

internal fun resolveMeasuredPageLength(
  length: Float,
  displayZoom: Float = 1f,
  density: Float = 0f,
): Float {
  val scaledLength = length * normalizeDisplayZoom(displayZoom)
  return scaledLength.resolveMeasuredLength(density = density, minimumPx = 1f)
}

private fun Float.resolveMeasuredLength(density: Float, minimumPx: Float): Float =
  if (density.isFinite() && density > 0f) {
    round(toDouble() * density.toDouble()).toFloat().coerceAtLeast(minimumPx) / density
  } else {
    this
  }
