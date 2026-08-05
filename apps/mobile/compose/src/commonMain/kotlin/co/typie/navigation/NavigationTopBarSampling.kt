package co.typie.navigation

import androidx.compose.runtime.Stable
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.geometry.takeOrElse
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.GraphicsContext
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.isSpecified
import androidx.compose.ui.graphics.layer.GraphicsLayer
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.roundToIntSize
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.ExperimentalHazeApi
import dev.chrisbanes.haze.InternalHazeApi
import dev.chrisbanes.haze.TrimMemoryLevel
import dev.chrisbanes.haze.VisualEffect
import dev.chrisbanes.haze.VisualEffectContext
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.TimeMark
import kotlin.time.TimeSource
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.yield

private const val LUMINANCE_ANALYSIS_WIDTH = 64
private const val LUMINANCE_ANALYSIS_HEIGHT = 16
private val MINIMUM_LUMINANCE_SAMPLE_INTERVAL = 100.milliseconds

private data class LuminanceSampleRegion(val renderWidth: Float, val top: Float, val height: Float)

internal class NavigationTopBarSampleToken(val owner: Any, val themeMode: ResolvedThemeMode)

private val DefaultNavigationTopBarSampleToken =
  NavigationTopBarSampleToken(Unit, ResolvedThemeMode.Light)

internal data class NavigationTopBarPixelSample(val pixels: IntArray, val width: Int)

@Stable
internal class NavigationTopBarSampleRequests : NavigationTopBarSampleRequester {
  private val mutableRequests =
    MutableSharedFlow<Unit>(extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)

  val flow: Flow<Unit> = mutableRequests.asSharedFlow()

  override fun requestSample() {
    mutableRequests.tryEmit(Unit)
  }
}

internal fun interface NavigationTopBarSampleRequester {
  fun requestSample()
}

internal val LocalNavigationTopBarSampleRequester =
  staticCompositionLocalOf<NavigationTopBarSampleRequester?> { null }

/**
 * Draws the Haze source unchanged and samples a tiny downscaled control-row image for luminance
 * policy. Only the 64 x 16 analysis layer crosses back to the CPU; the full scene never does.
 */
@Stable
@OptIn(ExperimentalHazeApi::class, InternalHazeApi::class)
internal class NavigationTopBarSamplingEffect(
  private val sampleTopInset: Dp,
  private val sampleHeight: Dp,
  private val backgroundColor: () -> Color,
  private val samplingEnabled: () -> Boolean = { true },
  private val sampleToken: () -> NavigationTopBarSampleToken? = {
    DefaultNavigationTopBarSampleToken
  },
  private val readPixels: (suspend (GraphicsLayer) -> NavigationTopBarPixelSample?)? = null,
  private val onPixels: (token: NavigationTopBarSampleToken, pixels: IntArray, width: Int) -> Unit,
) : VisualEffect {
  private var resolvedBackgroundColor = Color.Unspecified

  private var graphicsContext: GraphicsContext? = null
  private var contentLayer: GraphicsLayer? = null
  private var contentLayerSize: IntSize? = null
  private var analysisLayer: GraphicsLayer? = null
  private var sampleJob: Job? = null
  private var trailingSampleJob: Job? = null
  private var sampleRequestedWhileActive = false
  private var lastSampleStartedAt: TimeMark? = null
  private var visualEffectContext: VisualEffectContext? = null

  override fun attach(context: VisualEffectContext) {
    visualEffectContext = context
  }

  override fun update(context: VisualEffectContext) {
    visualEffectContext = context
    val currentBackgroundColor = backgroundColor()
    if (currentBackgroundColor != resolvedBackgroundColor) {
      resolvedBackgroundColor = currentBackgroundColor
      context.invalidateDraw()
    }
  }

  override fun DrawScope.draw(context: VisualEffectContext) {
    visualEffectContext = context
    val layerSize = context.layerSize.roundToIntSize()
    if (layerSize.width <= 0 || layerSize.height <= 0) return

    val layer = requireContentLayer(context, layerSize)
    layer.record(size = layerSize) {
      if (resolvedBackgroundColor.isSpecified) {
        drawRect(resolvedBackgroundColor)
      }

      val sourceOffset = context.layerOffset - context.position
      translate(sourceOffset.x, sourceOffset.y) {
        for (area in context.areas) {
          val areaPosition = Snapshot.withoutReadObservation {
            area.position.takeOrElse { Offset.Zero }
          }
          val areaLayer = Snapshot.withoutReadObservation {
            area.contentLayer
              ?.takeUnless { it.isReleased }
              ?.takeUnless { it.size.width <= 0 || it.size.height <= 0 }
          }

          if (areaLayer != null) {
            translate(areaPosition.x, areaPosition.y) { drawLayer(areaLayer) }
          }
        }
      }
    }

    if (samplingEnabled()) {
      scheduleSample(context = context, content = layer)
    }

    clipRect {
      val drawOffset = -context.layerOffset
      translate(drawOffset.x, drawOffset.y) { drawLayer(layer) }
    }
  }

  override fun detach(context: VisualEffectContext) {
    if (visualEffectContext === context) {
      visualEffectContext = null
    }
    releaseResources()
  }

  override fun onTrimMemory(context: VisualEffectContext, level: TrimMemoryLevel) {
    releaseResources()
    context.invalidateDraw()
  }

  override fun shouldClipToNodeBounds(): Boolean = true

  override fun shouldPreferClipToAreaBounds(): Boolean =
    resolvedBackgroundColor.isSpecified && resolvedBackgroundColor.alpha < 0.9f

  override fun calculateLayerBounds(rect: Rect, density: Density): Rect = rect

  internal fun requestSample() {
    visualEffectContext?.invalidateDraw()
  }

  private fun DrawScope.scheduleSample(context: VisualEffectContext, content: GraphicsLayer) {
    if (sampleJob?.isActive == true) {
      sampleRequestedWhileActive = true
      return
    }
    val previousStart = lastSampleStartedAt
    if (previousStart != null) {
      val elapsed = previousStart.elapsedNow()
      if (elapsed < MINIMUM_LUMINANCE_SAMPLE_INTERVAL) {
        scheduleTrailingSample(context, MINIMUM_LUMINANCE_SAMPLE_INTERVAL - elapsed)
        return
      }
    }
    trailingSampleJob?.cancel()
    trailingSampleJob = null

    val region = resolveSampleRegion(context) ?: return
    val token = sampleToken() ?: return

    val analysis = requireAnalysisLayer(context)
    analysis.record(size = IntSize(LUMINANCE_ANALYSIS_WIDTH, LUMINANCE_ANALYSIS_HEIGHT)) {
      val scaleX = LUMINANCE_ANALYSIS_WIDTH / region.renderWidth
      val scaleY = LUMINANCE_ANALYSIS_HEIGHT / region.height
      scale(scaleX = scaleX, scaleY = scaleY, pivot = Offset.Zero) {
        translate(left = -context.layerOffset.x, top = -(context.layerOffset.y + region.top)) {
          drawLayer(content)
        }
      }
    }

    lastSampleStartedAt = TimeSource.Monotonic.markNow()
    sampleJob =
      context.coroutineScope.launch {
        try {
          val sample =
            if (readPixels != null) {
              readPixels.invoke(analysis)
            } else {
              readAnalysisPixels(analysis)
            } ?: return@launch
          if (sampleToken() == token) {
            onPixels(token, sample.pixels, sample.width)
          }
        } finally {
          sampleJob = null
          if (sampleRequestedWhileActive && visualEffectContext === context) {
            sampleRequestedWhileActive = false
            scheduleTrailingSample(context)
          }
        }
      }
  }

  private fun resolveSampleRegion(context: VisualEffectContext): LuminanceSampleRegion? {
    val density = context.requireDensity()
    val renderWidth = context.size.width.takeIf { it.isFinite() && it > 0f } ?: return null
    val renderHeight = context.size.height.takeIf { it.isFinite() && it > 0f } ?: return null
    val topInsetPx = with(density) { sampleTopInset.toPx() }.coerceAtLeast(0f)
    val requestedHeightPx = with(density) { sampleHeight.toPx() }.coerceAtLeast(0f)
    val top = topInsetPx.coerceAtMost(renderHeight)
    val height = (top + requestedHeightPx).coerceAtMost(renderHeight) - top
    if (height <= 0f) return null
    return LuminanceSampleRegion(renderWidth = renderWidth, top = top, height = height)
  }

  private suspend fun readAnalysisPixels(analysis: GraphicsLayer): NavigationTopBarPixelSample? {
    // Leave the draw pass before snapshotting the recorded analysis layer.
    yield()
    val image =
      try {
        analysis.toImageBitmap()
      } catch (cancellation: CancellationException) {
        throw cancellation
      } catch (_: Exception) {
        return null
      }
    val pixels =
      try {
        IntArray(image.width * image.height).also { image.readPixels(it) }
      } catch (cancellation: CancellationException) {
        throw cancellation
      } catch (_: Exception) {
        return null
      }
    return NavigationTopBarPixelSample(pixels = pixels, width = image.width)
  }

  private fun scheduleTrailingSample(
    context: VisualEffectContext,
    delayDuration: Duration = remainingSampleInterval(),
  ) {
    if (trailingSampleJob?.isActive == true) return
    trailingSampleJob =
      context.coroutineScope.launch {
        if (delayDuration.isPositive()) delay(delayDuration)
        trailingSampleJob = null
        if (visualEffectContext === context) {
          context.invalidateDraw()
        }
      }
  }

  private fun remainingSampleInterval(): Duration {
    val previousStart = lastSampleStartedAt ?: return Duration.ZERO
    return (MINIMUM_LUMINANCE_SAMPLE_INTERVAL - previousStart.elapsedNow()).coerceAtLeast(
      Duration.ZERO
    )
  }

  private fun requireContentLayer(context: VisualEffectContext, size: IntSize): GraphicsLayer {
    val current = contentLayer
    if (current != null && !current.isReleased && contentLayerSize == size) {
      return current
    }

    releaseContentLayer()
    val currentGraphicsContext = context.requireGraphicsContext()
    return currentGraphicsContext.createGraphicsLayer().also {
      graphicsContext = currentGraphicsContext
      contentLayer = it
      contentLayerSize = size
    }
  }

  private fun requireAnalysisLayer(context: VisualEffectContext): GraphicsLayer {
    analysisLayer
      ?.takeUnless { it.isReleased }
      ?.let {
        return it
      }
    val currentGraphicsContext = graphicsContext ?: context.requireGraphicsContext()
    return currentGraphicsContext.createGraphicsLayer().also {
      graphicsContext = currentGraphicsContext
      analysisLayer = it
    }
  }

  private fun releaseResources() {
    sampleRequestedWhileActive = false
    sampleJob?.cancel()
    sampleJob = null
    trailingSampleJob?.cancel()
    trailingSampleJob = null
    lastSampleStartedAt = null
    releaseLayer(analysisLayer)
    analysisLayer = null
    releaseContentLayer()
  }

  private fun releaseContentLayer() {
    releaseLayer(contentLayer)
    contentLayer = null
    contentLayerSize = null
    if (analysisLayer == null) {
      graphicsContext = null
    }
  }

  private fun releaseLayer(layer: GraphicsLayer?) {
    val context = graphicsContext
    if (layer != null && context != null && !layer.isReleased) {
      context.releaseGraphicsLayer(layer)
    }
  }
}
