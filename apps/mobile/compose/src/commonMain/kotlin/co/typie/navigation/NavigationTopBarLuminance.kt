package co.typie.navigation

import co.typie.ui.component.topbar.TopBarForegroundStyle
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.math.abs
import kotlin.math.pow

private const val LIGHT_PIXEL_LUMINANCE_THRESHOLD = 0.421f
private const val LIGHT_BRIGHT_COVERAGE_THRESHOLD = 0.703f
private const val LIGHT_BRIGHT_COVERAGE_HYSTERESIS = 0.04f
private const val HORIZONTAL_CENTER_WEIGHT_BOOST = 0.85f
private const val SPATIAL_LUMINANCE_BLOCK_WIDTH = 4
private const val SPATIAL_LUMINANCE_BLOCK_HEIGHT = 16

private const val LIGHT_BACKDROP_OPACITY = 0.85f
private const val LIGHT_DARK_TINT_OPACITY = 0.25f

private const val DARK_LOW_BACKDROP_OPACITY = 0.85f
private const val DARK_MIDDLE_BACKDROP_OPACITY = 0.60f
private const val STYLE_STABILITY_MILLIS = 180L

private val DARK_LOW_LUMINANCE_THRESHOLD = encodedGrayRelativeLuminance(77)
private val DARK_LOW_LUMINANCE_ENTER_THRESHOLD = encodedGrayRelativeLuminance(85)
private val DARK_LOW_LUMINANCE_EXIT_THRESHOLD = encodedGrayRelativeLuminance(69)

internal data class NavigationTopBarLuminanceSample(
  val weightedMeanRelativeLuminance: Float,
  val weightedBrightCoverage: Float,
)

internal enum class NavigationTopBarContentLuminance {
  Dark,
  Bright,
}

internal data class NavigationTopBarLuminanceStyle(
  val backdropOpacity: Float,
  val darkTintOpacity: Float,
  val foregroundStyle: TopBarForegroundStyle,
)

internal fun analyzeNavigationTopBarPixels(
  pixels: IntArray,
  width: Int,
): NavigationTopBarLuminanceSample {
  require(width > 0) { "width must be positive" }
  require(pixels.isNotEmpty() && pixels.size % width == 0) {
    "pixels must contain complete non-empty rows"
  }

  val height = pixels.size / width
  val useSpatialBlocks =
    width >= SPATIAL_LUMINANCE_BLOCK_WIDTH && height >= SPATIAL_LUMINANCE_BLOCK_HEIGHT
  val blockWidth =
    if (useSpatialBlocks) {
      SPATIAL_LUMINANCE_BLOCK_WIDTH
    } else {
      1
    }
  val blockHeight =
    if (useSpatialBlocks) {
      SPATIAL_LUMINANCE_BLOCK_HEIGHT
    } else {
      1
    }
  val blockLuminances = FloatArray(blockWidth * blockHeight)
  var weightedLuminance = 0f
  var weightedBrightPixels = 0f
  var totalWeight = 0f

  for (blockTop in 0 until height step blockHeight) {
    for (blockLeft in 0 until width step blockWidth) {
      val blockRight = (blockLeft + blockWidth).coerceAtMost(width)
      val blockBottom = (blockTop + blockHeight).coerceAtMost(height)
      var count = 0
      for (y in blockTop until blockBottom) {
        for (x in blockLeft until blockRight) {
          blockLuminances[count++] = relativeLuminance(pixels[y * width + x])
        }
      }
      blockLuminances.sort(fromIndex = 0, toIndex = count)
      val luminance =
        if (count % 2 == 0) {
          (blockLuminances[count / 2 - 1] + blockLuminances[count / 2]) / 2f
        } else {
          blockLuminances[count / 2]
        }
      val weight =
        horizontalSampleWeight(normalizedX = (blockLeft + (blockRight - blockLeft) / 2f) / width)
      weightedLuminance += luminance * weight
      if (luminance >= LIGHT_PIXEL_LUMINANCE_THRESHOLD) {
        weightedBrightPixels += weight
      }
      totalWeight += weight
    }
  }

  return NavigationTopBarLuminanceSample(
    weightedMeanRelativeLuminance = weightedLuminance / totalWeight,
    weightedBrightCoverage = weightedBrightPixels / totalWeight,
  )
}

internal fun classifyNavigationTopBarContentLuminance(
  themeMode: ResolvedThemeMode,
  sample: NavigationTopBarLuminanceSample,
  current: NavigationTopBarContentLuminance? = null,
): NavigationTopBarContentLuminance =
  when (themeMode) {
    ResolvedThemeMode.Light -> {
      val brightCoverageThreshold =
        when {
          current == null -> LIGHT_BRIGHT_COVERAGE_THRESHOLD
          current == NavigationTopBarContentLuminance.Dark ->
            LIGHT_BRIGHT_COVERAGE_THRESHOLD + LIGHT_BRIGHT_COVERAGE_HYSTERESIS
          else -> LIGHT_BRIGHT_COVERAGE_THRESHOLD - LIGHT_BRIGHT_COVERAGE_HYSTERESIS
        }
      if (sample.weightedBrightCoverage >= brightCoverageThreshold) {
        NavigationTopBarContentLuminance.Bright
      } else {
        NavigationTopBarContentLuminance.Dark
      }
    }
    ResolvedThemeMode.Dark -> {
      val lowLuminanceThreshold =
        when {
          current == null -> DARK_LOW_LUMINANCE_THRESHOLD
          current == NavigationTopBarContentLuminance.Dark -> DARK_LOW_LUMINANCE_ENTER_THRESHOLD
          else -> DARK_LOW_LUMINANCE_EXIT_THRESHOLD
        }
      if (sample.weightedMeanRelativeLuminance <= lowLuminanceThreshold) {
        NavigationTopBarContentLuminance.Dark
      } else {
        NavigationTopBarContentLuminance.Bright
      }
    }
  }

internal fun navigationTopBarLuminanceStyle(
  themeMode: ResolvedThemeMode,
  contentLuminance: NavigationTopBarContentLuminance,
): NavigationTopBarLuminanceStyle =
  when (themeMode) {
    ResolvedThemeMode.Light ->
      if (contentLuminance == NavigationTopBarContentLuminance.Bright) {
        NavigationTopBarLuminanceStyle(
          backdropOpacity = LIGHT_BACKDROP_OPACITY,
          darkTintOpacity = 0f,
          foregroundStyle = TopBarForegroundStyle.Dark,
        )
      } else {
        NavigationTopBarLuminanceStyle(
          backdropOpacity = 0f,
          darkTintOpacity = LIGHT_DARK_TINT_OPACITY,
          foregroundStyle = TopBarForegroundStyle.Light,
        )
      }
    ResolvedThemeMode.Dark ->
      NavigationTopBarLuminanceStyle(
        backdropOpacity =
          if (contentLuminance == NavigationTopBarContentLuminance.Dark) {
            DARK_LOW_BACKDROP_OPACITY
          } else {
            DARK_MIDDLE_BACKDROP_OPACITY
          },
        darkTintOpacity = 0f,
        foregroundStyle = TopBarForegroundStyle.Light,
      )
  }

internal fun defaultNavigationTopBarContentLuminance(
  themeMode: ResolvedThemeMode
): NavigationTopBarContentLuminance =
  when (themeMode) {
    ResolvedThemeMode.Light -> NavigationTopBarContentLuminance.Bright
    ResolvedThemeMode.Dark -> NavigationTopBarContentLuminance.Dark
  }

internal fun defaultNavigationTopBarLuminanceStyle(
  themeMode: ResolvedThemeMode
): NavigationTopBarLuminanceStyle =
  navigationTopBarLuminanceStyle(
    themeMode = themeMode,
    contentLuminance = defaultNavigationTopBarContentLuminance(themeMode),
  )

internal fun navigationTopBarLuminanceTransitionDelayMillis(
  from: NavigationTopBarContentLuminance,
  to: NavigationTopBarContentLuminance,
): Long = if (from == to) 0L else STYLE_STABILITY_MILLIS

private fun horizontalSampleWeight(normalizedX: Float): Float {
  val centerProximity = 1f - abs(2f * normalizedX - 1f)
  return 1f + HORIZONTAL_CENTER_WEIGHT_BOOST * centerProximity
}

private fun relativeLuminance(argb: Int): Float {
  val red = srgbToLinear((argb ushr 16 and 0xFF) / 255f)
  val green = srgbToLinear((argb ushr 8 and 0xFF) / 255f)
  val blue = srgbToLinear((argb and 0xFF) / 255f)
  return 0.2126f * red + 0.7152f * green + 0.0722f * blue
}

private fun encodedGrayRelativeLuminance(gray: Int): Float = srgbToLinear(gray / 255f)

private fun srgbToLinear(value: Float): Float =
  if (value <= 0.04045f) {
    value / 12.92f
  } else {
    ((value + 0.055f) / 1.055f).pow(2.4f)
  }
