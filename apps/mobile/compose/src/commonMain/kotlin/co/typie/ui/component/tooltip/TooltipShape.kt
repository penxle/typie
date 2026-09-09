package co.typie.ui.component.tooltip

import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Matrix
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathOperation
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp

/** The web arrow is an 8dp rotated square with its exposed corner rounded by 2dp. */
internal data class TooltipShape(val anchorX: Float, val above: Boolean, val arrowExtent: Float) :
  Shape {
  override fun createOutline(
    size: Size,
    layoutDirection: LayoutDirection,
    density: Density,
  ): Outline =
    with(density) {
      val body =
        Path().apply {
          addRoundRect(
            RoundRect(
              Rect(0f, arrowExtent, size.width, size.height - arrowExtent),
              CornerRadius(4.dp.toPx()),
            )
          )
        }
      val halfArrow = 4.dp.toPx()
      val inset = (16.dp.toPx() + halfArrow).coerceAtMost(size.width / 2)
      val x = anchorX.coerceIn(inset, size.width - inset)
      val y = if (above) size.height - arrowExtent else arrowExtent
      val arrow =
        Path().apply {
          addRoundRect(
            RoundRect(
              -halfArrow,
              -halfArrow,
              halfArrow,
              halfArrow,
              topLeftCornerRadius = CornerRadius(2.dp.toPx()),
            )
          )
          transform(
            Matrix().apply {
              translate(x, y)
              rotateZ(if (above) 225f else 45f)
            }
          )
        }
      Outline.Generic(Path.combine(PathOperation.Union, body, arrow))
    }
}
