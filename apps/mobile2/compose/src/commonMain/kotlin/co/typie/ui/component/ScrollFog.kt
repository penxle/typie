package co.typie.ui.component

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.PaddingValues.Absolute
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

data class ScrollFogInsets(
  val top: Dp = 0.dp,
  val bottom: Dp = 0.dp,
  val left: Dp = 0.dp,
  val right: Dp = 0.dp,
)

fun ScrollFogInsets.toPaddingValues(): PaddingValues =
  Absolute(left = left, top = top, right = right, bottom = bottom)

fun Modifier.scrollFog(insets: ScrollFogInsets, color: Color): Modifier {
  if (insets == ScrollFogInsets()) {
    return this
  }

  val transparentColor = color.copy(alpha = 0f)
  return this.drawWithCache {
    val topHeight = insets.top.toPx()
    val bottomHeight = insets.bottom.toPx()
    val leftWidth = insets.left.toPx()
    val rightWidth = insets.right.toPx()

    val topBrush =
      if (topHeight > 0f) {
        Brush.verticalGradient(
          colorStops =
            arrayOf(0f to color, ScrollFogDefaults.SolidStop to color, 1f to transparentColor),
          startY = 0f,
          endY = topHeight,
        )
      } else {
        null
      }
    val bottomBrush =
      if (bottomHeight > 0f) {
        Brush.verticalGradient(
          colorStops =
            arrayOf(
              0f to transparentColor,
              (1f - ScrollFogDefaults.SolidStop) to color,
              1f to color,
            ),
          startY = size.height - bottomHeight,
          endY = size.height,
        )
      } else {
        null
      }
    val leftBrush =
      if (leftWidth > 0f) {
        Brush.horizontalGradient(
          colorStops =
            arrayOf(0f to color, ScrollFogDefaults.SolidStop to color, 1f to transparentColor),
          startX = 0f,
          endX = leftWidth,
        )
      } else {
        null
      }
    val rightBrush =
      if (rightWidth > 0f) {
        Brush.horizontalGradient(
          colorStops =
            arrayOf(
              0f to transparentColor,
              (1f - ScrollFogDefaults.SolidStop) to color,
              1f to color,
            ),
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
}

fun Modifier.bleedingScrollFog(insets: ScrollFogInsets, color: Color): Modifier {
  return this.bleedPadding(insets).scrollFog(insets, color)
}

@Composable
fun ScrollFog(insets: ScrollFogInsets, color: Color, modifier: Modifier = Modifier) {
  Box(modifier = modifier.scrollFog(insets, color))
}

private object ScrollFogDefaults {
  const val SolidStop: Float = 0.4f
}

@Composable
fun BleedingScrollFog(insets: ScrollFogInsets, color: Color, modifier: Modifier = Modifier) {
  Box(modifier = modifier.bleedingScrollFog(insets, color))
}
