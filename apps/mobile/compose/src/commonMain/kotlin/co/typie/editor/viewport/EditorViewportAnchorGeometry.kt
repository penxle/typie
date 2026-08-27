package co.typie.editor.viewport

import androidx.compose.ui.geometry.Offset
import co.typie.editor.VerticalSpan
import co.typie.editor.ffi.ResolvedViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.scroll.EditorScrollFrame

internal fun ResolvedViewportAnchor.toEditorViewportAnchorGeometry(
  frame: EditorScrollFrame,
  contentOriginY: Float,
): EditorViewportAnchorGeometry? {
  if (!contentOriginY.isFinite()) return null
  val zoom = frame.displayZoom.takeIf { it.isFinite() && it > 0f } ?: 1f
  val bodyGeometry = frame.bodyGeometry
  val contentWidth = maxOf(frame.visibleArea.viewport.width, bodyGeometry.pageColumnWidth)
  val pageColumnLeft = (contentWidth - bodyGeometry.pageColumnWidth) / 2f

  fun contentY(page: Int, y: Float): Float? {
    val pageTop = frame.pageContentTop(page) ?: return null
    return contentOriginY + pageTop + y * zoom
  }

  val pointY = contentY(point.pageIdx, point.y) ?: return null
  val pointX = pageColumnLeft + point.x * zoom
  if (!pointX.isFinite()) return null
  val rectSpan = rect?.let { pageRect ->
    val top = contentY(pageRect.pageIdx, pageRect.rect.y) ?: return@let null
    VerticalSpan(top = top, bottom = top + pageRect.rect.height * zoom)
  }
  return EditorViewportAnchorGeometry(pointY = pointY, pointX = pointX, rect = rectSpan)
}

internal fun resolveViewportAnchorContentOriginY(frame: EditorScrollFrame): Float {
  return (frame.headerHeight.takeIf(Float::isFinite) ?: 0f) + frame.bodyGeometry.topSpacerHeight
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

  val page = frame.pageAtContentY(relativeY) ?: return null
  val pageTop = frame.pageContentTop(page) ?: return null
  val pageSize = frame.state.pageSizes[page]
  val bodyGeometry = frame.bodyGeometry
  val contentWidth = maxOf(frame.visibleArea.viewport.width, bodyGeometry.pageColumnWidth)
  val pageColumnLeft = (contentWidth - bodyGeometry.pageColumnWidth) / 2f
  val viewportCenterX = scrollOffset.x + frame.visibleArea.viewport.width / 2f
  return ViewportAnchorPoint(
    pageIdx = page,
    x = ((viewportCenterX - pageColumnLeft) / zoom).coerceIn(0f, pageSize.width),
    y = ((relativeY - pageTop) / zoom).coerceIn(0f, pageSize.height),
  )
}
