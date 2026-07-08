package co.typie.editor.surface

import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.CacheDrawScope
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.ResolvedThemeMode

private class PageShadowLayer(val offsetY: Dp, val blur: Dp, val color: Color)

private val LightPageShadowLayers =
  listOf(
    PageShadowLayer(offsetY = 0.dp, blur = 3.dp, color = Color(0xFF18160F).copy(alpha = 0.04f)),
    PageShadowLayer(offsetY = 2.dp, blur = 8.dp, color = Color(0xFF18160F).copy(alpha = 0.03f)),
  )

private val DarkPageShadowLayers =
  listOf(
    PageShadowLayer(offsetY = 0.dp, blur = 3.dp, color = Color(0xFF000000).copy(alpha = 0.06f)),
    PageShadowLayer(offsetY = 2.dp, blur = 8.dp, color = Color(0xFF000000).copy(alpha = 0.05f)),
  )

// Gaussian step-edge alpha profile sampled at t = 0(inner)..1(outer).
private val EdgeProfile =
  floatArrayOf(0.977f, 0.933f, 0.841f, 0.691f, 0.5f, 0.309f, 0.159f, 0.067f, 0.023f)

private fun profileStops(color: Color, innerToOuter: Boolean): Array<Pair<Float, Color>> {
  val n = EdgeProfile.size
  return Array(n) { i ->
    val a = if (innerToOuter) EdgeProfile[i] else EdgeProfile[n - 1 - i]
    (i / (n - 1f)) to color.copy(alpha = color.alpha * a)
  }
}

internal fun Modifier.editorPageChromeShadow(themeMode: ResolvedThemeMode): Modifier {
  val layers =
    when (themeMode) {
      ResolvedThemeMode.Light -> LightPageShadowLayers
      ResolvedThemeMode.Dark -> DarkPageShadowLayers
    }
  return drawWithCache {
    val draws = layers.map { buildLayerDraw(it) }
    onDrawBehind { for (draw in draws) draw() }
  }
}

private fun CacheDrawScope.buildLayerDraw(layer: PageShadowLayer): DrawScope.() -> Unit {
  val r = layer.blur.toPx()
  val dy = layer.offsetY.toPx()
  val color = layer.color
  val rect = Rect(0f, dy, size.width, size.height + dy)
  val inner = Rect(rect.left + r, rect.top + r, rect.right - r, rect.bottom - r)
  if (r <= 0f || inner.width <= 0f || inner.height <= 0f) {
    return { drawRect(color, topLeft = rect.topLeft, size = rect.size) }
  }
  val outer = Rect(rect.left - r, rect.top - r, rect.right + r, rect.bottom + r)
  val d = 2 * r
  val inToOut = profileStops(color, innerToOuter = true)
  val outToIn = profileStops(color, innerToOuter = false)
  val top = Brush.verticalGradient(*outToIn, startY = outer.top, endY = inner.top)
  val bottom = Brush.verticalGradient(*inToOut, startY = inner.bottom, endY = outer.bottom)
  val left = Brush.horizontalGradient(*outToIn, startX = outer.left, endX = inner.left)
  val right = Brush.horizontalGradient(*inToOut, startX = inner.right, endX = outer.right)
  val corners =
    listOf(
        Offset(inner.left, inner.top) to Offset(outer.left, outer.top),
        Offset(inner.right, inner.top) to Offset(inner.right, outer.top),
        Offset(inner.left, inner.bottom) to Offset(outer.left, inner.bottom),
        Offset(inner.right, inner.bottom) to Offset(inner.right, inner.bottom),
      )
      .map { (center, topLeft) ->
        Brush.radialGradient(*inToOut, center = center, radius = d) to topLeft
      }
  return {
    drawRect(color, topLeft = inner.topLeft, size = inner.size)
    drawRect(top, topLeft = Offset(inner.left, outer.top), size = Size(inner.width, d))
    drawRect(bottom, topLeft = Offset(inner.left, inner.bottom), size = Size(inner.width, d))
    drawRect(left, topLeft = Offset(outer.left, inner.top), size = Size(d, inner.height))
    drawRect(right, topLeft = Offset(inner.right, inner.top), size = Size(d, inner.height))
    for ((brush, topLeft) in corners) {
      drawRect(brush, topLeft = topLeft, size = Size(d, d))
    }
  }
}
