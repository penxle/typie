package co.typie.editor.external

import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

internal const val IMAGE_MIN_WIDTH = 100f
internal const val IMAGE_MIN_PROPORTION = 10
internal const val IMAGE_MAX_PROPORTION = 100

internal data class ImageResizeWidthBounds(val min: Float, val max: Float)

internal fun imageResizeWidthBounds(
  boundsWidth: Float,
  originalWidth: Float,
): ImageResizeWidthBounds {
  val maxWidth = min(boundsWidth, if (originalWidth > 0f) originalWidth else boundsWidth)
  val requestedMin = max(boundsWidth * (IMAGE_MIN_PROPORTION / 100f), IMAGE_MIN_WIDTH)
  val minWidth = min(requestedMin, maxWidth)
  return ImageResizeWidthBounds(min = minWidth, max = maxWidth)
}

internal fun clampImageResizeWidth(width: Float, boundsWidth: Float, originalWidth: Float): Float {
  val bounds = imageResizeWidthBounds(boundsWidth = boundsWidth, originalWidth = originalWidth)
  return width.coerceIn(bounds.min, bounds.max)
}

internal fun imageResizeProportionForWidth(width: Float, boundsWidth: Float): Int =
  ((width / boundsWidth) * 100).roundToInt().coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)

internal fun imageResizeWidthForProportion(
  proportion: Float,
  boundsWidth: Float,
  originalWidth: Float,
): Float =
  clampImageResizeWidth(
    width = boundsWidth * (proportion / 100f),
    boundsWidth = boundsWidth,
    originalWidth = originalWidth,
  )

internal fun imageResizeDisplayPercent(
  proportion: Float,
  boundsWidth: Float,
  originalWidth: Float,
): Int {
  val maxWidth = imageResizeWidthBounds(boundsWidth, originalWidth).max
  if (maxWidth <= 0f) {
    return IMAGE_MAX_PROPORTION
  }
  val width = imageResizeWidthForProportion(proportion, boundsWidth, originalWidth)
  return ((width / maxWidth) * 100)
    .roundToInt()
    .coerceIn(IMAGE_MIN_PROPORTION, IMAGE_MAX_PROPORTION)
}

internal fun imageResizeProportionRange(boundsWidth: Float, originalWidth: Float): IntRange {
  val bounds = imageResizeWidthBounds(boundsWidth = boundsWidth, originalWidth = originalWidth)
  return imageResizeProportionForWidth(bounds.min, boundsWidth)..imageResizeProportionForWidth(
      bounds.max,
      boundsWidth,
    )
}
