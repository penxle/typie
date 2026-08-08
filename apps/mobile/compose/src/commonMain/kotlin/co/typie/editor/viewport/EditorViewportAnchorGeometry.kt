package co.typie.editor.viewport

import co.typie.editor.VerticalSpan
import co.typie.editor.body.resolvePageContentTop
import co.typie.editor.ffi.ResolvedViewportAnchor
import co.typie.editor.ffi.ViewportAnchorPoint
import co.typie.editor.scroll.EditorScrollFrame

internal fun ResolvedViewportAnchor.toEditorViewportAnchorGeometry(
  frame: EditorScrollFrame,
  contentOriginY: Float = frame.headerHeight + frame.editorBounds.y,
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

internal fun viewportCenterAnchorPoint(
  frame: EditorScrollFrame,
  scrollY: Float,
): ViewportAnchorPoint? {
  if (frame.state.pageSizes.isEmpty()) return null
  val contentOriginY = frame.headerHeight + frame.editorBounds.y
  val viewportCenterY =
    (frame.visibleArea.visibleViewportTop + frame.visibleArea.visibleViewportBottom) / 2f
  if (!contentOriginY.isFinite() || !scrollY.isFinite() || !viewportCenterY.isFinite()) return null
  val zoom = frame.displayZoom.takeIf { it.isFinite() && it > 0f } ?: 1f
  val relativeY = scrollY + viewportCenterY - contentOriginY

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
  return ViewportAnchorPoint(
    pageIdx = page,
    x = frame.state.pageSizes[page].width / 2f,
    y = (relativeY - pageTop) / zoom,
  )
}
