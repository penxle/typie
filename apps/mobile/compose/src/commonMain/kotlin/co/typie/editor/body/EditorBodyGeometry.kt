package co.typie.editor.body

import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.scroll.EditorVisibleArea

private const val ContinuousTopSpacerHeight = 40f
private const val PaginatedTopSpacerHeight = 0f

internal data class EditorBodyGeometry(
  val pageColumnWidth: Float,
  val visibleBodySize: Size,
  val minimumBodyHeight: Float,
  val topSpacerHeight: Float,
)

internal fun resolveEditorBodyGeometry(
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float = 1f,
): EditorBodyGeometry {
  val effectiveDisplayZoom =
    if (displayZoom.isFinite() && displayZoom > 0f) {
      displayZoom
    } else {
      1f
    }
  val visibleBodySize = visibleArea.visibleBodySize
  val maxPageWidth = resolveEditorPageWidth(pageSizes) ?: 0f
  val pageColumnWidth =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous ->
        when {
          // Continuous.maxWidth caps content; engine page sizes also include its page margins.
          maxPageWidth > 0f -> maxPageWidth
          else -> visibleBodySize.width
        }
      is EditorDocumentLayoutSpec.Paginated ->
        when {
          layoutSpec.pageWidth > 0f -> layoutSpec.pageWidth * effectiveDisplayZoom
          maxPageWidth > 0f -> maxPageWidth * effectiveDisplayZoom
          else -> visibleBodySize.width
        }
    }
  val minimumBodyHeight =
    resolveMinimumBodyHeight(
      viewportHeight = visibleArea.viewport.height,
      headerHeight = visibleArea.headerHeight,
      bottomOcclusion = visibleArea.bottomOcclusion,
    )

  return EditorBodyGeometry(
    pageColumnWidth = pageColumnWidth,
    visibleBodySize = visibleBodySize,
    minimumBodyHeight = minimumBodyHeight,
    topSpacerHeight =
      when (layoutSpec) {
        is EditorDocumentLayoutSpec.Continuous -> ContinuousTopSpacerHeight
        is EditorDocumentLayoutSpec.Paginated -> PaginatedTopSpacerHeight
      },
  )
}

internal fun resolveEditorPageWidth(pageSizes: List<PageSize>): Float? =
  pageSizes.asSequence().map(PageSize::width).filter { it.isFinite() && it > 0f }.maxOrNull()

internal fun resolveMinimumBodyHeight(
  viewportHeight: Float,
  headerHeight: Float,
  bottomOcclusion: Float,
): Float = (viewportHeight - headerHeight - bottomOcclusion).coerceAtLeast(0f)
