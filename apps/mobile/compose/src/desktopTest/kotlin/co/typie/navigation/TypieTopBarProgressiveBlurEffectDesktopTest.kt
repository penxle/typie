package co.typie.navigation

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

private const val MaximumContinuousLuminanceStep = 0.08f

@OptIn(ExperimentalTestApi::class)
class TypieTopBarProgressiveBlurEffectDesktopTest {
  @Test
  fun nestedBlurLayerRedrawRefreshesMeasuredLuminance() = runComposeUiTest {
    val sourceColor = mutableStateOf(Color.White)
    val measurements = mutableListOf<NavigationTopBarMeasuredContentLuminance>()

    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        Box(Modifier.size(width = 160.dp, height = 120.dp)) {
          Box(Modifier.fillMaxSize().hazeSource(hazeState)) {
            Canvas(Modifier.fillMaxSize().blur(16.dp)) { drawRect(sourceColor.value) }
          }
          NavigationTopBarBackdrop(
            hazeState = hazeState,
            style = {
              NavigationTopBarBackdropStyle(background = Color.Transparent, presence = 1f)
            },
            blurEnabled = false,
            luminanceMode = NavigationTopBarLuminanceMode.Live(Unit),
            themeMode = ResolvedThemeMode.Light,
            onMeasuredContentLuminance = measurements::add,
          )
        }
      }
    }

    waitUntil(timeoutMillis = 5_000) {
      measurements.any { it.contentLuminance == NavigationTopBarContentLuminance.Bright }
    }
    runOnIdle {
      measurements.clear()
      sourceColor.value = Color.Black
    }
    waitUntil(timeoutMillis = 5_000) {
      measurements.any { it.contentLuminance == NavigationTopBarContentLuminance.Dark }
    }
  }

  @Test
  fun frozenSeedIsNotPublishedUntilDestinationPixelsAreMeasured() = runComposeUiTest {
    val luminanceMode =
      mutableStateOf<NavigationTopBarLuminanceMode>(
        NavigationTopBarLuminanceMode.Frozen(NavigationTopBarContentLuminance.Dark)
      )
    val measurements = mutableListOf<NavigationTopBarMeasuredContentLuminance>()

    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        Box(Modifier.size(width = 160.dp, height = 120.dp)) {
          Box(Modifier.fillMaxSize().background(Color.Black).hazeSource(hazeState))
          NavigationTopBarBackdrop(
            hazeState = hazeState,
            style = {
              NavigationTopBarBackdropStyle(background = Color.Transparent, presence = 1f)
            },
            blurEnabled = false,
            luminanceMode = luminanceMode.value,
            themeMode = ResolvedThemeMode.Light,
            onMeasuredContentLuminance = measurements::add,
          )
        }
      }
    }
    waitForIdle()
    assertTrue(measurements.isEmpty(), "frozen seed was published: $measurements")

    luminanceMode.value = NavigationTopBarLuminanceMode.Live("destination")
    waitUntil(timeoutMillis = 5_000) { measurements.isNotEmpty() }

    val measured = measurements.first()
    assertEquals("destination", measured.token.owner)
    assertEquals(ResolvedThemeMode.Light, measured.token.themeMode)
    assertEquals(NavigationTopBarContentLuminance.Dark, measured.contentLuminance)
  }

  @Test
  fun hiddenTopBarDoesNotKeepBackdropEffectComposed() = runComposeUiTest {
    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 0f } }

      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        NavigationTopBarBackdrop(
          hazeState = hazeState,
          style = { NavigationTopBarBackdropStyle(background = Color.Black, presence = 1f) },
          luminanceMode = NavigationTopBarLuminanceMode.Live(Unit),
          themeMode = ResolvedThemeMode.Light,
        )
      }
    }
    waitForIdle()

    onNodeWithTag(NavigationTopBarBackdropTestTag).assertDoesNotExist()
  }

  @Test
  fun navigationTopBarOpacityChangesContinuouslyThroughFade() = runComposeUiTest {
    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        Box(Modifier.size(width = 160.dp, height = 120.dp)) {
          Canvas(Modifier.fillMaxSize().hazeSource(hazeState)) {
            drawRect(Color.Black)
            for (x in 0 until size.width.toInt() step 2) {
              drawRect(
                color = Color.White,
                topLeft = Offset(x = x.toFloat(), y = 0f),
                size = Size(width = 1f, height = size.height),
              )
            }
          }
          NavigationTopBarBackdrop(
            hazeState = hazeState,
            style = {
              NavigationTopBarBackdropStyle(background = Color.Transparent, presence = 1f)
            },
            luminanceMode = NavigationTopBarLuminanceMode.Live(Unit),
            themeMode = ResolvedThemeMode.Light,
            modifier = Modifier,
          )
        }
      }
    }
    waitForIdle()

    val pixels = onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
    val sampleX = pixels.width / 2
    val luminance = (2 until pixels.height - 2).map { y -> pixels[sampleX, y].luminance() }
    val steps = luminance.zipWithNext { first, second -> abs(second - first) }
    val maximumStep = steps.maxOrNull() ?: 0f
    val maximumRow = steps.indexOf(maximumStep) + 2
    val largestSteps =
      steps
        .withIndex()
        .sortedByDescending { it.value }
        .take(8)
        .joinToString { (index, value) -> "${index + 2}:$value" }

    assertTrue(
      maximumStep <= MaximumContinuousLuminanceStep,
      "progressive fade has a luminance step of $maximumStep at row $maximumRow " +
        "(height=${pixels.height}, largest=[$largestSteps])",
    )
  }

  @Test
  fun navigationTopBarBlurCrossfadesOverTheLiveBackdrop() = runComposeUiTest {
    val blurEnabled = mutableStateOf(false)

    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Light,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        Box(Modifier.size(width = 160.dp, height = 120.dp)) {
          Canvas(Modifier.fillMaxSize().hazeSource(hazeState)) {
            drawRect(Color.Black)
            for (x in 0 until size.width.toInt() step 2) {
              drawRect(
                color = Color.White,
                topLeft = Offset(x = x.toFloat(), y = 0f),
                size = Size(width = 1f, height = size.height),
              )
            }
          }
          NavigationTopBarBackdrop(
            hazeState = hazeState,
            style = {
              NavigationTopBarBackdropStyle(background = Color.Transparent, presence = 1f)
            },
            blurEnabled = blurEnabled.value,
            luminanceMode =
              NavigationTopBarLuminanceMode.Frozen(NavigationTopBarContentLuminance.Bright),
            themeMode = ResolvedThemeMode.Light,
          )
        }
      }
    }
    waitForIdle()

    val clearPixels = onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
    val clearContrast = clearPixels.averageHorizontalLuminanceDelta(y = 4)

    mainClock.autoAdvance = false
    blurEnabled.value = true
    mainClock.advanceTimeByFrame()
    mainClock.advanceTimeBy(32)
    waitForIdle()

    val midpointPixels =
      onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
    val midpointContrast = midpointPixels.averageHorizontalLuminanceDelta(y = 4)

    mainClock.advanceTimeBy(250)
    waitForIdle()

    val blurredPixels = onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
    val blurredContrast = blurredPixels.averageHorizontalLuminanceDelta(y = 4)

    assertTrue(
      midpointContrast < clearContrast * 0.9f && midpointContrast > blurredContrast * 1.1f,
      "midpoint contrast=$midpointContrast, blurred contrast=$blurredContrast, " +
        "clear contrast=$clearContrast",
    )
    assertTrue(
      blurredContrast < clearContrast * 0.5f,
      "blurred contrast=$blurredContrast, clear contrast=$clearContrast",
    )
  }

  @Test
  fun darkTopBarFadeBecomesLighterOverBrightContent() = runComposeUiTest {
    val blackOffsetY = mutableIntStateOf(0)
    val sampleRequests = NavigationTopBarSampleRequests()
    var controlHeightPx = 0

    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(
        LocalThemeMode provides ResolvedThemeMode.Dark,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        val topPadding = TopBarDefaults.topPadding()
        val density = LocalDensity.current
        val sampleTopPx = with(density) { topPadding.roundToPx() }
        controlHeightPx = with(density) { TopBarDefaults.Height.roundToPx() }
        Box(Modifier.size(width = 160.dp, height = 120.dp)) {
          Box(Modifier.fillMaxSize().hazeSource(hazeState)) {
            Box(Modifier.fillMaxSize().background(Color.White))
            Box(
              Modifier.offset { IntOffset(x = 0, y = sampleTopPx + blackOffsetY.intValue) }
                .fillMaxWidth()
                .height(TopBarDefaults.Height)
                .background(Color.Black)
            )
          }
          NavigationTopBarBackdrop(
            hazeState = hazeState,
            style = { NavigationTopBarBackdropStyle(background = Color.Black, presence = 1f) },
            blurEnabled = false,
            luminanceMode = NavigationTopBarLuminanceMode.Live(Unit),
            themeMode = ResolvedThemeMode.Dark,
            sampleRequests = sampleRequests.flow,
          )
        }
      }
    }
    waitForIdle()

    val darkPixels = onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
    val sampleY = (darkPixels.height - 14).coerceAtLeast(0)
    val darkFadeLuminance = darkPixels[darkPixels.width / 2, sampleY].luminance()

    blackOffsetY.intValue = -controlHeightPx
    waitForIdle()
    sampleRequests.requestSample()
    waitUntil(timeoutMillis = 5_000) {
      val pixels = onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
      pixels[pixels.width / 2, sampleY].luminance() > darkFadeLuminance + 0.2f
    }
  }

  @Test
  fun adaptiveFadeResamplesAfterThemeChange() = runComposeUiTest {
    val themeMode = mutableStateOf(ResolvedThemeMode.Dark)
    val sourceColor = mutableStateOf(Color.White)
    val sampleRequests = NavigationTopBarSampleRequests()

    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(
        LocalThemeMode provides themeMode.value,
        LocalTopBarAnimationSource provides topBarState,
      ) {
        Box(Modifier.size(width = 160.dp, height = 120.dp)) {
          Box(Modifier.fillMaxSize().hazeSource(hazeState)) {
            Box(Modifier.fillMaxSize().background(sourceColor.value))
          }
          NavigationTopBarBackdrop(
            hazeState = hazeState,
            style = {
              NavigationTopBarBackdropStyle(
                background =
                  if (themeMode.value == ResolvedThemeMode.Light) Color.White else Color.Black,
                presence = 1f,
              )
            },
            blurEnabled = false,
            luminanceMode = NavigationTopBarLuminanceMode.Live(Unit),
            themeMode = themeMode.value,
            sampleRequests = sampleRequests.flow,
          )
        }
      }
    }
    waitForIdle()

    runOnIdle {
      themeMode.value = ResolvedThemeMode.Light
      sourceColor.value = Color.Black
    }
    sampleRequests.requestSample()

    waitUntil(timeoutMillis = 5_000) {
      val pixels = onNodeWithTag(NavigationTopBarBackdropTestTag).captureToImage().toPixelMap()
      pixels[pixels.width / 2, 4].luminance() < 0.2f
    }
  }
}

private fun androidx.compose.ui.graphics.PixelMap.averageHorizontalLuminanceDelta(y: Int): Float {
  val deltas =
    (0 until width - 1).map { x -> abs(this[x, y].luminance() - this[x + 1, y].luminance()) }
  return deltas.average().toFloat()
}
