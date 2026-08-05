package co.typie.navigation

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.ExperimentalHazeApi
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.hazeSource
import dev.chrisbanes.haze.rememberHazeState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred

private const val SamplingEffectTag = "sampling-effect"

@OptIn(ExperimentalTestApi::class, ExperimentalHazeApi::class)
class NavigationTopBarSamplingEffectDesktopTest {
  @Test
  fun inFlightReadbackIsDiscardedWhenSampleSessionChanges() = runComposeUiTest {
    val tokenA = NavigationTopBarSampleToken(owner = "A", themeMode = ResolvedThemeMode.Light)
    val tokenB = NavigationTopBarSampleToken(owner = "B", themeMode = ResolvedThemeMode.Dark)
    val sampleToken = mutableStateOf(tokenA)
    val firstReadStarted = CompletableDeferred<Unit>()
    val releaseFirstRead = CompletableDeferred<Unit>()
    val secondReadStarted = CompletableDeferred<Unit>()
    val releaseSecondRead = CompletableDeferred<Unit>()
    val accepted = mutableListOf<NavigationTopBarSampleToken>()
    var readCount = 0
    lateinit var samplingEffect: NavigationTopBarSamplingEffect

    setContent {
      val hazeState = rememberHazeState()
      val effect = remember {
        NavigationTopBarSamplingEffect(
          sampleTopInset = 0.dp,
          sampleHeight = 40.dp,
          backgroundColor = { Color.Black },
          sampleToken = { sampleToken.value },
          readPixels = {
            when (readCount++) {
              0 -> {
                firstReadStarted.complete(Unit)
                releaseFirstRead.await()
              }
              1 -> {
                secondReadStarted.complete(Unit)
                releaseSecondRead.await()
              }
            }
            NavigationTopBarPixelSample(IntArray(64 * 16), width = 64)
          },
          onPixels = { token, _, _ -> accepted += token },
        )
      }
      samplingEffect = effect

      Box(Modifier.size(width = 160.dp, height = 40.dp)) {
        Box(Modifier.fillMaxSize().background(Color.White).hazeSource(hazeState))
        Box(Modifier.fillMaxSize().hazeEffect(hazeState) { visualEffect = effect })
      }
    }

    waitUntil(timeoutMillis = 5_000) { firstReadStarted.isCompleted }
    sampleToken.value = tokenB
    samplingEffect.requestSample()
    releaseFirstRead.complete(Unit)

    waitUntil(timeoutMillis = 5_000) { secondReadStarted.isCompleted }
    assertTrue(accepted.isEmpty(), "stale result was accepted: $accepted")

    releaseSecondRead.complete(Unit)
    waitUntil(timeoutMillis = 5_000) { accepted.isNotEmpty() }
    assertEquals(tokenB, accepted.first())
  }

  @Test
  fun disabledSamplingSkipsReadbackUntilReenabled() = runComposeUiTest {
    val samplingEnabled = mutableStateOf(false)
    var sampleCount = 0
    lateinit var samplingEffect: NavigationTopBarSamplingEffect

    setContent {
      val hazeState = rememberHazeState()
      val effect = remember {
        NavigationTopBarSamplingEffect(
          sampleTopInset = 0.dp,
          sampleHeight = 40.dp,
          backgroundColor = { Color.Black },
          samplingEnabled = { samplingEnabled.value },
          onPixels = { _, _, _ -> sampleCount++ },
        )
      }
      samplingEffect = effect

      Box(Modifier.size(width = 160.dp, height = 40.dp)) {
        Box(Modifier.fillMaxSize().background(Color.White).hazeSource(hazeState))
        Box(Modifier.fillMaxSize().hazeEffect(hazeState) { visualEffect = effect })
      }
    }
    waitForIdle()
    samplingEffect.requestSample()
    waitForIdle()
    assertTrue(sampleCount == 0, "disabled sample count=$sampleCount")

    samplingEnabled.value = true
    samplingEffect.requestSample()
    waitUntil(timeoutMillis = 5_000) { sampleCount > 0 }
  }

  @Test
  fun requestedReadbackRefreshesWhenHazeSourceMovesDuringPlacement() = runComposeUiTest {
    val sourceOffsetY = mutableIntStateOf(60)
    var sample: PixelSample? = null
    lateinit var samplingEffect: NavigationTopBarSamplingEffect

    setContent {
      val hazeState = rememberHazeState()
      val effect = remember {
        NavigationTopBarSamplingEffect(
          sampleTopInset = 0.dp,
          sampleHeight = 40.dp,
          backgroundColor = { Color.White },
          onPixels = { _, pixels, width -> sample = PixelSample(pixels, width) },
        )
      }
      samplingEffect = effect

      Box(Modifier.size(width = 160.dp, height = 100.dp)) {
        Box(Modifier.fillMaxSize().hazeSource(hazeState)) {
          Box(
            Modifier.offset { IntOffset(x = 0, y = sourceOffsetY.intValue) }
              .width(160.dp)
              .height(40.dp)
              .background(Color.Black)
          )
        }
        Box(Modifier.fillMaxSize().hazeEffect(hazeState) { visualEffect = effect })
      }
    }

    waitUntil(timeoutMillis = 5_000) {
      sample?.averageEncodedBrightness()?.let { it > 0.92f } == true
    }
    sourceOffsetY.intValue = 0
    waitForIdle()
    samplingEffect.requestSample()
    waitUntil(timeoutMillis = 5_000) {
      sample?.averageEncodedBrightness()?.let { it < 0.08f } == true
    }
  }

  @Test
  fun readbackRefreshesWhenHazeSourceChanges() = runComposeUiTest {
    val sourceColor = mutableStateOf(Color.Black)
    var sample: PixelSample? = null

    setContent {
      val hazeState = rememberHazeState()
      val effect = remember {
        NavigationTopBarSamplingEffect(
          sampleTopInset = 0.dp,
          sampleHeight = 40.dp,
          backgroundColor = { Color.Magenta },
          onPixels = { _, pixels, width -> sample = PixelSample(pixels, width) },
        )
      }

      Box(Modifier.size(width = 160.dp, height = 40.dp)) {
        Canvas(Modifier.fillMaxSize().hazeSource(hazeState)) { drawRect(sourceColor.value) }
        Box(Modifier.fillMaxSize().hazeEffect(hazeState) { visualEffect = effect })
      }
    }

    waitUntil(timeoutMillis = 5_000) {
      sample?.averageEncodedBrightness()?.let { it < 0.08f } == true
    }
    sourceColor.value = Color.White
    waitUntil(timeoutMillis = 5_000) {
      sample?.averageEncodedBrightness()?.let { it > 0.92f } == true
    }
  }

  @Test
  fun readbackDownsamplesOnlyConfiguredControlRow() = runComposeUiTest {
    var sample: PixelSample? = null

    setContent {
      val hazeState = rememberHazeState()
      val effect = remember {
        NavigationTopBarSamplingEffect(
          sampleTopInset = 20.dp,
          sampleHeight = 40.dp,
          backgroundColor = { Color.Magenta },
          onPixels = { _, pixels, width -> sample = PixelSample(pixels, width) },
        )
      }

      Box(Modifier.size(width = 160.dp, height = 100.dp)) {
        Canvas(Modifier.fillMaxSize().hazeSource(hazeState)) {
          drawRect(Color.Red, size = Size(size.width, 20.dp.toPx()))
          drawRect(
            Color.Black,
            topLeft = Offset(0f, 20.dp.toPx()),
            size = Size(size.width / 2f, 40.dp.toPx()),
          )
          drawRect(
            Color.White,
            topLeft = Offset(size.width / 2f, 20.dp.toPx()),
            size = Size(size.width / 2f, 40.dp.toPx()),
          )
          drawRect(
            Color.Blue,
            topLeft = Offset(0f, 60.dp.toPx()),
            size = Size(size.width, size.height - 60.dp.toPx()),
          )
        }
        Box(
          Modifier.fillMaxSize().testTag(SamplingEffectTag).hazeEffect(hazeState) {
            visualEffect = effect
          }
        )
      }
    }

    waitUntil(timeoutMillis = 5_000) { sample != null }
    val resolved = requireNotNull(sample)
    val height = resolved.pixels.size / resolved.width
    assertEquals(64, resolved.width)
    assertEquals(16, height)
    val left = averageRgb(resolved, xRange = 0 until resolved.width / 2, height = height)
    val right =
      averageRgb(resolved, xRange = resolved.width / 2 until resolved.width, height = height)

    assertTrue(left.red < 0.08f && left.green < 0.08f && left.blue < 0.08f, "left=$left")
    assertTrue(right.red > 0.92f && right.green > 0.92f && right.blue > 0.92f, "right=$right")

    val rendered = onNodeWithTag(SamplingEffectTag).captureToImage().toPixelMap()
    val boundaryY = rendered.height / 2
    val leftBoundaryLuminance = rendered[rendered.width / 2 - 1, boundaryY].luminance()
    val rightBoundaryLuminance = rendered[rendered.width / 2, boundaryY].luminance()
    assertTrue(leftBoundaryLuminance < 0.08f, "left boundary=$leftBoundaryLuminance")
    assertTrue(rightBoundaryLuminance > 0.92f, "right boundary=$rightBoundaryLuminance")
  }

  private fun averageRgb(sample: PixelSample, xRange: IntRange, height: Int): Rgb {
    var red = 0L
    var green = 0L
    var blue = 0L
    var count = 0
    for (y in 0 until height) {
      for (x in xRange) {
        val argb = sample.pixels[y * sample.width + x]
        red += argb ushr 16 and 0xFF
        green += argb ushr 8 and 0xFF
        blue += argb and 0xFF
        count += 1
      }
    }
    return Rgb(
      red = red / count.toFloat() / 255f,
      green = green / count.toFloat() / 255f,
      blue = blue / count.toFloat() / 255f,
    )
  }

  private fun PixelSample.averageEncodedBrightness(): Float {
    var total = 0L
    pixels.forEach { argb ->
      total += (argb ushr 16 and 0xFF) + (argb ushr 8 and 0xFF) + (argb and 0xFF)
    }
    return total / pixels.size.toFloat() / (3f * 255f)
  }

  private data class PixelSample(val pixels: IntArray, val width: Int)

  private data class Rgb(val red: Float, val green: Float, val blue: Float)
}
