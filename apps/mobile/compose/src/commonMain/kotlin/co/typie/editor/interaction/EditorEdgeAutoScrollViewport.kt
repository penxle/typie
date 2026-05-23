package co.typie.editor.interaction

import androidx.compose.ui.geometry.Rect
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState

internal data class EditorEdgeAutoScrollViewport(val rect: Rect, val density: Float)

internal fun resolveEditorEdgeAutoScrollViewport(
  uiState: EditorUiState,
  visibleArea: EditorVisibleArea,
  viewportState: EditorViewportState,
  density: Float,
): EditorEdgeAutoScrollViewport? {
  val editorBounds = uiState.editorBoundsInContainer
  if (density <= 0f || !editorBounds.isValid) {
    return null
  }

  val scrollOffset = viewportState.scrollOffset
  val left = (scrollOffset.x - editorBounds.x) * density
  val top =
    (scrollOffset.y + visibleArea.visibleViewportTop - visibleArea.headerHeight - editorBounds.y) *
      density
  val right = left + visibleArea.viewport.width * density
  val bottom =
    (scrollOffset.y + visibleArea.visibleViewportBottom -
      visibleArea.headerHeight -
      editorBounds.y) * density
  if (right <= left || bottom <= top) {
    return null
  }

  return EditorEdgeAutoScrollViewport(
    rect = Rect(left = left, top = top, right = right, bottom = bottom),
    density = density,
  )
}
