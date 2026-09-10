package co.typie.editor.external

import androidx.compose.ui.geometry.Size
import kotlin.math.min

internal const val IMAGE_MIN_PROPORTION = 10
internal const val IMAGE_MAX_PROPORTION = 100

internal fun imageResizeMaxSize(
  boundsWidth: Float,
  originalWidth: Float,
  imageRatio: Float,
  maxHeight: Float?,
): Size {
  val width = min(boundsWidth, originalWidth)
  val height = min(width / imageRatio, maxHeight ?: Float.POSITIVE_INFINITY)
  return Size(width = min(width, height * imageRatio), height = height)
}

internal fun imageResizeSize(proportion: Float, maxSize: Size): Size =
  maxSize *
    (proportion.coerceIn(IMAGE_MIN_PROPORTION.toFloat(), IMAGE_MAX_PROPORTION.toFloat()) / 100f)
