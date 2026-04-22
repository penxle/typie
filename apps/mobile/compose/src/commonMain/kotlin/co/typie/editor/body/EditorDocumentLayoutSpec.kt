package co.typie.editor.body

import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Size as PageSize

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
    is LayoutMode.Continuous -> EditorDocumentLayoutSpec.Continuous(maxWidth = maxWidth)
    is LayoutMode.Paginated ->
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = pageWidth,
        pageHeight = pageHeight,
        pageMarginTop = pageMarginTop,
        pageMarginBottom = pageMarginBottom,
        pageMarginLeft = pageMarginLeft,
        pageMarginRight = pageMarginRight,
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
): Float {
  val effectiveDisplayZoom = normalizeDisplayZoom(displayZoom)
  val pageGap =
    when (this) {
      is EditorDocumentLayoutSpec.Paginated -> resolvePaginatedPageGap(effectiveDisplayZoom)
      is EditorDocumentLayoutSpec.Continuous -> 0f
    }
  return pageSizes.foldIndexed(0f) { index, total, size ->
    total + size.height * effectiveDisplayZoom + if (index < pageSizes.lastIndex) pageGap else 0f
  }
}

private fun normalizeDisplayZoom(displayZoom: Float): Float =
  if (displayZoom.isFinite() && displayZoom > 0f) {
    displayZoom
  } else {
    1f
  }
