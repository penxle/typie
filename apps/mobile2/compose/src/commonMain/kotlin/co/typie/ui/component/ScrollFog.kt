package co.typie.ui.component

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.CacheDrawModifierNode
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.node.DelegatingNode
import androidx.compose.ui.node.ModifierNodeElement

fun Modifier.scrollFog(padding: PaddingValues, color: Color): Modifier =
  this then ScrollFogElement(padding, color)

fun Modifier.bleedingScrollFog(padding: PaddingValues, color: Color): Modifier =
  this.bleedPadding(padding).scrollFog(padding, color)

private data class ScrollFogElement(private val padding: PaddingValues, private val color: Color) :
  ModifierNodeElement<ScrollFogNode>() {
  override fun create(): ScrollFogNode = ScrollFogNode(padding, color)

  override fun update(node: ScrollFogNode) {
    node.padding = padding
    node.color = color
    node.invalidateDrawCache()
  }
}

private class ScrollFogNode(var padding: PaddingValues, var color: Color) : DelegatingNode() {
  private val cacheNode =
    delegate(
      CacheDrawModifierNode {
        val topHeight = padding.calculateTopPadding().toPx()
        val bottomHeight = padding.calculateBottomPadding().toPx()
        val leftWidth = padding.calculateLeftPadding(layoutDirection).toPx()
        val rightWidth = padding.calculateRightPadding(layoutDirection).toPx()
        val transparentColor = color.copy(alpha = 0f)

        val topBrush =
          if (topHeight > 0f) {
            Brush.verticalGradient(
              colorStops = arrayOf(0f to color, SolidStop to color, 1f to transparentColor),
              startY = 0f,
              endY = topHeight,
            )
          } else {
            null
          }
        val bottomBrush =
          if (bottomHeight > 0f) {
            Brush.verticalGradient(
              colorStops = arrayOf(0f to transparentColor, (1f - SolidStop) to color, 1f to color),
              startY = size.height - bottomHeight,
              endY = size.height,
            )
          } else {
            null
          }
        val leftBrush =
          if (leftWidth > 0f) {
            Brush.horizontalGradient(
              colorStops = arrayOf(0f to color, SolidStop to color, 1f to transparentColor),
              startX = 0f,
              endX = leftWidth,
            )
          } else {
            null
          }
        val rightBrush =
          if (rightWidth > 0f) {
            Brush.horizontalGradient(
              colorStops = arrayOf(0f to transparentColor, (1f - SolidStop) to color, 1f to color),
              startX = size.width - rightWidth,
              endX = size.width,
            )
          } else {
            null
          }

        onDrawWithContent {
          drawContent()

          topBrush?.let {
            drawRect(
              brush = it,
              topLeft = Offset.Zero,
              size = Size(width = size.width, height = topHeight),
            )
          }
          bottomBrush?.let {
            drawRect(
              brush = it,
              topLeft = Offset(x = 0f, y = size.height - bottomHeight),
              size = Size(width = size.width, height = bottomHeight),
            )
          }
          leftBrush?.let {
            drawRect(
              brush = it,
              topLeft = Offset.Zero,
              size = Size(width = leftWidth, height = size.height),
            )
          }
          rightBrush?.let {
            drawRect(
              brush = it,
              topLeft = Offset(x = size.width - rightWidth, y = 0f),
              size = Size(width = rightWidth, height = size.height),
            )
          }
        }
      }
    )

  fun invalidateDrawCache() {
    cacheNode.invalidateDrawCache()
  }
}

private const val SolidStop: Float = 0.4f
