package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint

internal interface EditorInteractionGeometry {
  val density: Float

  fun resolveInteractionPosition(positionInSurface: Offset): Offset?

  fun isTapEligible(positionInSurface: Offset): Boolean

  fun resolvePoint(positionInNode: Offset): PagePoint?

  fun resolvePagePosition(page: Int, x: Float, y: Float): Offset?

  fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport?
}
