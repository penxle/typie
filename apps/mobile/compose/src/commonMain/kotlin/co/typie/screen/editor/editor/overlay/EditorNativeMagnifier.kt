package co.typie.screen.editor.editor.overlay

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.graphics.layer.GraphicsLayer
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.unit.dp
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ui.theme.AppTheme
import kotlin.math.max

internal val EditorMagnifierWidth = 144.dp
internal val EditorMagnifierHeight = 80.dp
internal val EditorMagnifierVerticalOffset = 60.dp
internal const val EditorMagnifierZoom = 1.3f
internal val EditorSoftwareMagnifierShadowBlur = 8.dp
internal val EditorSoftwareMagnifierShadowOffsetY = 2.dp
internal const val EditorSoftwareMagnifierShadowAlpha = 0.26f

internal data class EditorMagnifierPlacement(
  val sourceCenter: Offset,
  val magnifierCenter: Offset,
  val topLeft: Offset,
)

internal fun resolveEditorMagnifierPlacement(
  focalPosition: Offset,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  density: Float,
): EditorMagnifierPlacement? {
  if (
    !focalPosition.isSpecified ||
      overlaySize.width <= 0f ||
      overlaySize.height <= 0f ||
      density <= 0f
  ) {
    return null
  }

  val widthPx = EditorMagnifierWidth.value * density
  val heightPx = EditorMagnifierHeight.value * density
  val verticalOffsetPx = EditorMagnifierVerticalOffset.value * density
  val visibleTopPx = visibleArea.visibleViewportTop * density
  val visibleBottomPx = visibleArea.visibleViewportBottom * density
  val minCenterX = widthPx / 2f
  val maxCenterX = max(minCenterX, overlaySize.width - widthPx / 2f)
  val clampedCenterX = focalPosition.x.coerceIn(minCenterX, maxCenterX)
  val showBelow = focalPosition.y < visibleTopPx + heightPx + verticalOffsetPx
  val preferredTop =
    if (showBelow) {
      focalPosition.y + verticalOffsetPx
    } else {
      focalPosition.y - verticalOffsetPx - heightPx
    }
  val minTop = visibleTopPx.coerceAtLeast(0f)
  val maxTop = max(minTop, visibleBottomPx - heightPx)
  val top = preferredTop.coerceIn(minTop, maxTop)
  val left = (clampedCenterX - widthPx / 2f).coerceAtLeast(0f)

  return EditorMagnifierPlacement(
    sourceCenter = focalPosition,
    magnifierCenter = Offset(clampedCenterX, top + heightPx / 2f),
    topLeft = Offset(left, top),
  )
}

internal expect val EditorNativeMagnifierAvailable: Boolean

internal expect fun Modifier.editorNativeMagnifier(placement: EditorMagnifierPlacement?): Modifier

@Composable
internal fun Modifier.editorSoftwareMagnifierLens(
  sourceLayer: GraphicsLayer,
  placement: EditorMagnifierPlacement?,
): Modifier =
  if (EditorNativeMagnifierAvailable) {
    this
  } else {
    editorSoftwareMagnifier(
      sourceLayer = sourceLayer,
      placement = placement,
      borderColor = AppTheme.colors.borderDefault,
      shadowColor = Color.Black.copy(alpha = EditorSoftwareMagnifierShadowAlpha),
      backgroundColor = AppTheme.colors.surfaceDefault,
    )
  }

internal fun Modifier.editorSoftwareMagnifierSource(
  sourceLayer: GraphicsLayer,
  active: Boolean,
): Modifier =
  if (EditorNativeMagnifierAvailable || !active) {
    this
  } else {
    drawWithContent {
      sourceLayer.record { this@drawWithContent.drawContent() }
      drawLayer(sourceLayer)
    }
  }

private fun Modifier.editorSoftwareMagnifier(
  sourceLayer: GraphicsLayer,
  placement: EditorMagnifierPlacement?,
  borderColor: Color,
  shadowColor: Color,
  backgroundColor: Color,
): Modifier = drawWithContent {
  drawContent()

  if (placement == null) {
    return@drawWithContent
  }

  val magnifierSize =
    Size(width = EditorMagnifierWidth.toPx(), height = EditorMagnifierHeight.toPx())
  val cornerRadius = CornerRadius(magnifierSize.height / 2f, magnifierSize.height / 2f)
  val shadowBlur = EditorSoftwareMagnifierShadowBlur.toPx()
  val shadowOffsetY = EditorSoftwareMagnifierShadowOffsetY.toPx()
  val clipPath =
    Path().apply {
      addRoundRect(
        RoundRect(
          rect = Rect(offset = placement.topLeft, size = magnifierSize),
          cornerRadius = cornerRadius,
        )
      )
    }

  drawRoundRect(
    color = shadowColor.copy(alpha = shadowColor.alpha * 0.24f),
    topLeft = placement.topLeft + Offset(x = -shadowBlur / 2f, y = shadowOffsetY - shadowBlur / 2f),
    size =
      Size(width = magnifierSize.width + shadowBlur, height = magnifierSize.height + shadowBlur),
    cornerRadius =
      CornerRadius(x = cornerRadius.x + shadowBlur / 2f, y = cornerRadius.y + shadowBlur / 2f),
  )
  drawRoundRect(
    color = shadowColor.copy(alpha = shadowColor.alpha * 0.32f),
    topLeft = placement.topLeft + Offset(x = -shadowBlur / 4f, y = shadowOffsetY - shadowBlur / 4f),
    size =
      Size(
        width = magnifierSize.width + shadowBlur / 2f,
        height = magnifierSize.height + shadowBlur / 2f,
      ),
    cornerRadius =
      CornerRadius(x = cornerRadius.x + shadowBlur / 4f, y = cornerRadius.y + shadowBlur / 4f),
  )
  drawRoundRect(
    color = shadowColor.copy(alpha = shadowColor.alpha * 0.42f),
    topLeft = placement.topLeft + Offset(x = 0f, y = shadowOffsetY),
    size = magnifierSize,
    cornerRadius = cornerRadius,
  )
  drawRoundRect(
    color = backgroundColor,
    topLeft = placement.topLeft,
    size = magnifierSize,
    cornerRadius = cornerRadius,
  )
  clipPath(clipPath) {
    withTransform({
      translate(left = placement.magnifierCenter.x, top = placement.magnifierCenter.y)
      scale(scaleX = EditorMagnifierZoom, scaleY = EditorMagnifierZoom, pivot = Offset.Zero)
      translate(left = -placement.sourceCenter.x, top = -placement.sourceCenter.y)
    }) {
      drawLayer(sourceLayer)
    }
  }
  drawRoundRect(
    color = borderColor,
    topLeft = placement.topLeft,
    size = magnifierSize,
    cornerRadius = cornerRadius,
    style = Stroke(width = 1.dp.toPx()),
  )
}
