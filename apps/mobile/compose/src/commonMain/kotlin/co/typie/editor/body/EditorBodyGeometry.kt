package co.typie.editor.body

import androidx.compose.ui.geometry.Size
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.scroll.EditorVisibleArea
import kotlin.math.max
import kotlin.math.min

private const val ContinuousTopSpacerHeight = 40f
private const val PaginatedTopSpacerHeight = 0f

internal data class EditorBodyGeometry(
  val pageColumnWidth: Float,
  val visibleBodySize: Size,
  val visibleExtensionSize: Size,
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
  val visibleExtensionSize = visibleArea.visibleExtensionSize
  val maxPageWidth = pageSizes.maxOfOrNull(PageSize::width) ?: 0f
  val preferredPageWidth =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous -> layoutSpec.maxWidth
      is EditorDocumentLayoutSpec.Paginated -> layoutSpec.pageWidth
    }
  val pageColumnWidth =
    when (layoutSpec) {
      is EditorDocumentLayoutSpec.Continuous ->
        when {
          preferredPageWidth > 0f && visibleExtensionSize.width > 0f ->
            min(preferredPageWidth, visibleExtensionSize.width)
          preferredPageWidth > 0f -> preferredPageWidth
          maxPageWidth > 0f && visibleExtensionSize.width > 0f ->
            min(maxPageWidth, visibleExtensionSize.width)
          maxPageWidth > 0f -> maxPageWidth
          else -> visibleExtensionSize.width
        }
      is EditorDocumentLayoutSpec.Paginated ->
        when {
          preferredPageWidth > 0f -> preferredPageWidth * effectiveDisplayZoom
          maxPageWidth > 0f -> maxPageWidth * effectiveDisplayZoom
          else -> visibleExtensionSize.width
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
    visibleExtensionSize = visibleExtensionSize,
    minimumBodyHeight = minimumBodyHeight,
    topSpacerHeight =
      when (layoutSpec) {
        is EditorDocumentLayoutSpec.Continuous -> ContinuousTopSpacerHeight
        is EditorDocumentLayoutSpec.Paginated -> PaginatedTopSpacerHeight
      },
  )
}

internal fun resolveMinimumBodyHeight(
  viewportHeight: Float,
  headerHeight: Float,
  bottomOcclusion: Float,
): Float = max(0f, viewportHeight - headerHeight - bottomOcclusion)
