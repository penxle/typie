package co.typie.ui.shape

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection
import co.typie.ext.toPx
import kotlin.math.min

/**
 * iOS-style continuous corner (squircle) shape. Based on
 * UIBezierPath(roundedRect:cornerRadius:style: .continuous).
 */
class SquircleShape(private val cornerRadius: Dp) : Shape {

  override fun createOutline(
    size: Size,
    layoutDirection: LayoutDirection,
    density: Density,
  ): Outline {
    val radiusPx = cornerRadius.toPx(density)
    return Outline.Generic(squirclePath(size.width, size.height, radiusPx))
  }
}

private fun squirclePath(width: Float, height: Float, radius: Float): Path {
  val r = min(radius, min(width, height) / 2f)
  if (r <= 0f) {
    return Path().apply { addRect(Rect(0f, 0f, width, height)) }
  }

  // iOS continuous corner control point ratios (~1.528665 magic number)
  val a = min(1.528665f * r, min(width, height) / 2f)
  val b = 0.000000f * r
  val c = 0.127000f * r
  val d = 0.250000f * r
  val e = 0.439000f * r
  val f = 0.556000f * r

  val path = Path()

  // Top-right corner
  path.moveTo(width / 2f, 0f)
  path.lineTo(width - a, 0f)
  path.cubicTo(width - f, b, width - e, c, width - d, d)
  path.cubicTo(width - c, e, width - b, f, width, a)

  // Bottom-right corner
  path.lineTo(width, height - a)
  path.cubicTo(width - b, height - f, width - c, height - e, width - d, height - d)
  path.cubicTo(width - e, height - c, width - f, height - b, width - a, height)

  // Bottom-left corner
  path.lineTo(a, height)
  path.cubicTo(f, height - b, e, height - c, d, height - d)
  path.cubicTo(c, height - e, b, height - f, 0f, height - a)

  // Top-left corner
  path.lineTo(0f, a)
  path.cubicTo(b, f, c, e, d, d)
  path.cubicTo(e, c, f, b, a, 0f)

  path.close()
  return path
}
