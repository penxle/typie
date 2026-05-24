package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint

internal interface EditorInteractionGeometry {
  fun resolvePoint(positionInNode: Offset): PagePoint?

  fun resolvePagePosition(page: Int, x: Float, y: Float): Offset?

  fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport?
}
