package co.typie.ui.utils

import androidx.compose.ui.graphics.Color
import kotlin.math.roundToInt

internal fun Color.toHexRgbString(): String {
  fun channel(value: Float): String {
    return (value * 255f).roundToInt().coerceIn(0, 255).toString(16).padStart(2, '0').uppercase()
  }

  return "#${channel(red)}${channel(green)}${channel(blue)}"
}
