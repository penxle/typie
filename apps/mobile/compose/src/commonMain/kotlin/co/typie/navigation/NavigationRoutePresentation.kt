package co.typie.navigation

import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.ClipOp
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.graphicsLayer

/** Keeps per-frame animation state reads inside layer and draw callbacks. */
internal class NavigationRoutePresentation(
  val translationX: () -> Float = { 0f },
  val translationY: () -> Float = { 0f },
  val alpha: () -> Float = { 1f },
  val clipShape: (() -> Shape?)? = null,
)

internal fun Modifier.navigationRoutePresentation(
  presentation: NavigationRoutePresentation
): Modifier = graphicsLayer {
  translationX = presentation.translationX()
  translationY = presentation.translationY()
  alpha = presentation.alpha()
  val resolvedClipShape = presentation.clipShape?.invoke()
  clip = resolvedClipShape != null
  if (resolvedClipShape != null) {
    shape = resolvedClipShape
  }
}

internal fun Modifier.excludeNavigationRouteCoverage(
  presentation: NavigationRoutePresentation
): Modifier {
  val clipShape = presentation.clipShape ?: return this
  return drawWithContent {
    val resolvedClipShape = clipShape()
    if (resolvedClipShape == null) {
      drawContent()
      return@drawWithContent
    }
    val outline = resolvedClipShape.createOutline(size, layoutDirection, this)
    val coveragePath =
      when (outline) {
        is Outline.Generic -> Path().apply { addPath(outline.path) }
        is Outline.Rectangle -> Path().apply { addRect(outline.rect) }
        is Outline.Rounded -> Path().apply { addRoundRect(outline.roundRect) }
      }
    coveragePath.translate(Offset(x = presentation.translationX(), y = presentation.translationY()))
    clipPath(coveragePath, clipOp = ClipOp.Difference) { this@drawWithContent.drawContent() }
  }
}
