package co.typie.navigation

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.TopBarState
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertTrue

private const val MaximumContinuousLuminanceStep = 0.08f

@OptIn(ExperimentalTestApi::class)
class TypieTopBarProgressiveBlurEffectDesktopTest {
  @Test
  fun hiddenTopBarDoesNotKeepBackdropEffectComposed() = runComposeUiTest {
    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 0f } }

      CompositionLocalProvider(LocalTopBarAnimationSource provides topBarState) {
        NavigationTopBarBackdrop(
          hazeState = hazeState,
          style = { NavigationTopBarBackdropStyle(background = Color.Black, presence = 1f) },
        )
      }
    }
    waitForIdle()

    onNodeWithTag(NavigationTopBarBackdropTestTag).assertDoesNotExist()
  }

  @Test
  fun navigationTopBarBlurChangesContinuouslyThroughFade() = runComposeUiTest {
    setContent {
      val hazeState = rememberHazeState()
      val topBarState = remember { TopBarState().apply { animatedAlpha = 1f } }

      CompositionLocalProvider(LocalTopBarAnimationSource provides topBarState) {
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
      "progressive blur has a luminance step of $maximumStep at row $maximumRow " +
        "(height=${pixels.height}, largest=[$largestSteps])",
    )
  }
}
