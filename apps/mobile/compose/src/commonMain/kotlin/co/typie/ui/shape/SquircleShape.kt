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
class SquircleShape(private val cornerRadius: Dp, private val smoothing: Float = 1f) : Shape {

  override fun createOutline(
    size: Size,
    layoutDirection: LayoutDirection,
    density: Density,
  ): Outline {
    val radiusPx = cornerRadius.toPx(density)
    return Outline.Generic(squirclePath(size.width, size.height, radiusPx, smoothing))
  }
}

private fun squirclePath(width: Float, height: Float, radius: Float, smoothing: Float): Path {
  val r = min(radius, min(width, height) / 2f)
  if (r <= 0f) {
    return Path().apply { addRect(Rect(0f, 0f, width, height)) }
  }

  // Two cubic segments per corner interpolate from a circular arc to the existing
  // continuous corner. Both endpoints retain the same path topology during a morph.
  val t = smoothing.coerceIn(0f, 1f)
  fun interpolate(round: Float, continuous: Float) = (round + (continuous - round) * t) * r
  val a = min(interpolate(1f, 1.528665f), min(width, height) / 2f)
  val b = 0f
  val c = interpolate(0.10535684f, 0.127f)
  val d = interpolate(0.29289322f, 0.250f)
  val e = interpolate(0.48042956f, 0.439f)
  val f = interpolate(0.7347835f, 0.556f)

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
