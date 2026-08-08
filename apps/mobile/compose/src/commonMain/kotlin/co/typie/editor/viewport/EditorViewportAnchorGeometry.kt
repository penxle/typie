package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import co.typie.editor.VerticalSpan
import co.typie.editor.body.resolveEditorBodyGeometry
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ffi.ResolvedViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.scroll.EditorScrollFrame

internal fun ResolvedViewportAnchor.toEditorViewportAnchorGeometry(
  frame: EditorScrollFrame,
  contentOriginY: Float,
): EditorViewportAnchorGeometry? {
  if (!contentOriginY.isFinite()) return null
  val zoom = frame.displayZoom.takeIf { it.isFinite() && it > 0f } ?: 1f

  fun contentY(page: Int, y: Float): Float? {
    val pageTop =
      frame.layoutSpec.resolvePageContentTop(
        page = page,
        pageSizes = frame.state.pageSizes,
        displayZoom = zoom,
        density = frame.density,
      ) ?: return null
    return contentOriginY + pageTop + y * zoom
  }

  val pointY = contentY(point.pageIdx, point.y) ?: return null
  val rectSpan = rect?.let { pageRect ->
    val top = contentY(pageRect.pageIdx, pageRect.rect.y) ?: return@let null
    VerticalSpan(top = top, bottom = top + pageRect.rect.height * zoom)
  }
  return EditorViewportAnchorGeometry(pointY = pointY, rect = rectSpan)
}

internal fun resolveViewportAnchorContentOriginY(frame: EditorScrollFrame): Float {
  val bodyGeometry =
    resolveEditorBodyGeometry(
      visibleArea = frame.visibleArea,
      layoutSpec = frame.layoutSpec,
      pageSizes = frame.state.pageSizes,
      displayZoom = frame.displayZoom,
    )
  return (frame.headerHeight.takeIf(Float::isFinite) ?: 0f) + bodyGeometry.topSpacerHeight
}

internal fun viewportCenterAnchorPoint(
  frame: EditorScrollFrame,
  scrollOffset: Offset,
  contentOriginY: Float,
): ViewportAnchorPoint? {
  if (frame.state.pageSizes.isEmpty()) return null
  val viewportCenterY =
    (frame.visibleArea.visibleViewportTop + frame.visibleArea.visibleViewportBottom) / 2f
  if (
    !contentOriginY.isFinite() ||
      !scrollOffset.x.isFinite() ||
      !scrollOffset.y.isFinite() ||
      !viewportCenterY.isFinite()
  ) {
    return null
  }
  val zoom = frame.displayZoom.takeIf { it.isFinite() && it > 0f } ?: 1f
  val relativeY = scrollOffset.y + viewportCenterY - contentOriginY

  var page = frame.state.pageSizes.lastIndex
  for (candidate in frame.state.pageSizes.indices) {
    val nextTop =
      frame.layoutSpec.resolvePageContentTop(
        page = candidate + 1,
        pageSizes = frame.state.pageSizes,
        displayZoom = zoom,
        density = frame.density,
      )
    if (nextTop == null || relativeY < nextTop) {
      page = candidate
      break
    }
  }
  val pageTop =
    frame.layoutSpec.resolvePageContentTop(
      page = page,
      pageSizes = frame.state.pageSizes,
      displayZoom = zoom,
      density = frame.density,
    ) ?: return null
  val bodyGeometry =
    resolveEditorBodyGeometry(
      visibleArea = frame.visibleArea,
      layoutSpec = frame.layoutSpec,
      pageSizes = frame.state.pageSizes,
      displayZoom = zoom,
    )
  val contentWidth = maxOf(frame.visibleArea.viewport.width, bodyGeometry.pageColumnWidth)
  val pageColumnLeft = (contentWidth - bodyGeometry.pageColumnWidth) / 2f
  val viewportCenterX = scrollOffset.x + frame.visibleArea.viewport.width / 2f
  val pageWidth = frame.state.pageSizes[page].width
  return ViewportAnchorPoint(
    pageIdx = page,
    x = ((viewportCenterX - pageColumnLeft) / zoom).coerceIn(0f, pageWidth),
    y = (relativeY - pageTop) / zoom,
  )
}
