package co.typie.editor.ext

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.positionInRoot

internal fun LayoutCoordinates.unclippedBoundsInRoot(): Rect {
  val position = positionInRoot()
  return Rect(
    left = position.x,
    top = position.y,
    right = position.x + size.width,
    bottom = position.y + size.height,
  )
}
