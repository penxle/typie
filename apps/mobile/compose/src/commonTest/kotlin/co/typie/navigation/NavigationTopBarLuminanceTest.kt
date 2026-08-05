package co.typie.navigation

import co.typie.ui.component.topbar.TopBarForegroundStyle
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals

class NavigationTopBarLuminanceTest {
  @Test
  fun lightUniformGraySwitchesBetween173And174() {
    val low =
      resolvedStyle(
        themeMode = ResolvedThemeMode.Light,
        sample = analyzeNavigationTopBarPixels(uniformPixels(gray = 173), width = 1),
      )
    val high =
      resolvedStyle(
        themeMode = ResolvedThemeMode.Light,
        sample = analyzeNavigationTopBarPixels(uniformPixels(gray = 174), width = 1),
      )

    assertEquals(0f, low.backdropOpacity)
    assertEquals(0.25f, low.darkTintOpacity)
    assertEquals(TopBarForegroundStyle.Light, low.foregroundStyle)
    assertEquals(0.85f, high.backdropOpacity)
    assertEquals(0f, high.darkTintOpacity)
    assertEquals(TopBarForegroundStyle.Dark, high.foregroundStyle)
  }

  @Test
  fun lightMixedCoverageUsesBroadCenterWeightedSampling() {
    val leadingLow = styleForWhiteRun(start = 0, width = 269)
    val leadingHigh = styleForWhiteRun(start = 0, width = 270)
    val centerLow = styleForWhiteRun(start = (ContentWidth - 254) / 2, width = 254)
    val centerHigh = styleForWhiteRun(start = (ContentWidth - 255) / 2, width = 255)

    assertEquals(0.25f, leadingLow.darkTintOpacity)
    assertEquals(0f, leadingHigh.darkTintOpacity)
    assertEquals(0.25f, centerLow.darkTintOpacity)
    assertEquals(0f, centerHigh.darkTintOpacity)
  }

  @Test
  fun darkUniformGrayUsesTwoBackdropOpacities() {
    assertEquals(0.85f, darkStyle(gray = 77).backdropOpacity)
    assertEquals(0.60f, darkStyle(gray = 78).backdropOpacity)
    assertEquals(0.60f, darkStyle(gray = 221).backdropOpacity)
    assertEquals(0.60f, darkStyle(gray = 222).backdropOpacity)
  }

  @Test
  fun luminanceBoundariesUseTheCurrentClassificationAsHysteresis() {
    val lightDark = contentLuminanceForWhiteRun(start = 0, width = 269)
    val lightBright = contentLuminanceForWhiteRun(start = 0, width = 270)
    val darkDark = darkContentLuminance(gray = 77)
    val darkBright = darkContentLuminance(gray = 78)

    assertEquals(
      lightDark,
      contentLuminanceForWhiteRun(start = 0, width = 270, current = lightDark),
    )
    assertEquals(
      lightBright,
      contentLuminanceForWhiteRun(start = 0, width = 269, current = lightBright),
    )
    assertEquals(darkDark, darkContentLuminance(gray = 78, current = darkDark))
    assertEquals(darkBright, darkContentLuminance(gray = 77, current = darkBright))
  }

  @Test
  fun thinDarkTextDoesNotChangeTheUnderlyingBrightImageTone() {
    val sample =
      analyzeNavigationTopBarPixels(
        pixels = thinTextPatternPixels(background = White, foreground = Black),
        width = SampleWidth,
      )

    val lightStyle = resolvedStyle(themeMode = ResolvedThemeMode.Light, sample = sample)
    val darkStyle = resolvedStyle(themeMode = ResolvedThemeMode.Dark, sample = sample)

    assertEquals(0f, lightStyle.darkTintOpacity)
    assertEquals(0.60f, darkStyle.backdropOpacity)
  }

  @Test
  fun narrowHorizontalStripesAreTreatedAsTextureInsteadOfDarkRegions() {
    val horizontalSample =
      analyzeNavigationTopBarPixels(pixels = horizontalStripePatternPixels(), width = SampleWidth)

    assertEquals(
      0f,
      resolvedStyle(themeMode = ResolvedThemeMode.Light, sample = horizontalSample).darkTintOpacity,
    )
  }

  @Test
  fun denseDarkPatternStillChangesTheUnderlyingBrightImageTone() {
    val sample =
      analyzeNavigationTopBarPixels(
        pixels = denseTextPatternPixels(background = White, foreground = Black),
        width = SampleWidth,
      )

    val lightStyle = resolvedStyle(themeMode = ResolvedThemeMode.Light, sample = sample)
    val darkStyle = resolvedStyle(themeMode = ResolvedThemeMode.Dark, sample = sample)

    assertEquals(0.25f, lightStyle.darkTintOpacity)
    assertEquals(0.85f, darkStyle.backdropOpacity)
  }

  @Test
  fun everyStyleChangeWaitsForAStableCandidate() {
    val dark = NavigationTopBarContentLuminance.Dark
    val bright = NavigationTopBarContentLuminance.Bright

    assertEquals(180L, navigationTopBarLuminanceTransitionDelayMillis(from = dark, to = bright))
    assertEquals(180L, navigationTopBarLuminanceTransitionDelayMillis(from = bright, to = dark))
    assertEquals(0L, navigationTopBarLuminanceTransitionDelayMillis(from = dark, to = dark))
  }

  private fun styleForWhiteRun(
    start: Int,
    width: Int,
    current: NavigationTopBarContentLuminance? = null,
  ): NavigationTopBarLuminanceStyle {
    val contentLuminance = contentLuminanceForWhiteRun(start, width, current)
    return navigationTopBarLuminanceStyle(ResolvedThemeMode.Light, contentLuminance)
  }

  private fun contentLuminanceForWhiteRun(
    start: Int,
    width: Int,
    current: NavigationTopBarContentLuminance? = null,
  ): NavigationTopBarContentLuminance {
    val pixels = IntArray(ContentWidth) { Black }
    for (x in start until start + width) {
      pixels[x] = White
    }
    return classifyNavigationTopBarContentLuminance(
      themeMode = ResolvedThemeMode.Light,
      sample = analyzeNavigationTopBarPixels(pixels = pixels, width = ContentWidth),
      current = current,
    )
  }

  private fun darkStyle(
    gray: Int,
    current: NavigationTopBarContentLuminance? = null,
  ): NavigationTopBarLuminanceStyle =
    navigationTopBarLuminanceStyle(
      themeMode = ResolvedThemeMode.Dark,
      contentLuminance = darkContentLuminance(gray, current),
    )

  private fun darkContentLuminance(
    gray: Int,
    current: NavigationTopBarContentLuminance? = null,
  ): NavigationTopBarContentLuminance =
    classifyNavigationTopBarContentLuminance(
      themeMode = ResolvedThemeMode.Dark,
      sample = analyzeNavigationTopBarPixels(uniformPixels(gray), width = 1),
      current = current,
    )

  private fun resolvedStyle(
    themeMode: ResolvedThemeMode,
    sample: NavigationTopBarLuminanceSample,
  ): NavigationTopBarLuminanceStyle =
    navigationTopBarLuminanceStyle(
      themeMode = themeMode,
      contentLuminance =
        classifyNavigationTopBarContentLuminance(themeMode = themeMode, sample = sample),
    )

  private fun uniformPixels(gray: Int): IntArray = intArrayOf(argb(gray, gray, gray))

  private fun thinTextPatternPixels(background: Int, foreground: Int): IntArray =
    IntArray(SampleWidth * SampleHeight) { index ->
      val x = index % SampleWidth
      val y = index / SampleWidth
      val isGlyphStroke = x % 4 == 1 || (y % 8 == 3 && x % 8 in 1..5)
      if (isGlyphStroke) foreground else background
    }

  private fun denseTextPatternPixels(background: Int, foreground: Int): IntArray =
    IntArray(SampleWidth * SampleHeight) { index ->
      val x = index % SampleWidth
      if (x % 4 < 3) foreground else background
    }

  private fun horizontalStripePatternPixels(): IntArray =
    IntArray(SampleWidth * SampleHeight) { index ->
      val y = index / SampleWidth
      if (y % 8 < 4) Black else White
    }

  private companion object {
    const val ContentWidth = 402
    const val SampleWidth = 64
    const val SampleHeight = 16
    val Black = argb(0, 0, 0)
    val White = argb(255, 255, 255)

    fun argb(red: Int, green: Int, blue: Int): Int =
      (0xFF shl 24) or (red shl 16) or (green shl 8) or blue
  }
}
