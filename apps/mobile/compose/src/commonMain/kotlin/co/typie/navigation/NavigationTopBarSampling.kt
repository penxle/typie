package co.typie.navigation

import androidx.compose.runtime.Stable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.GraphicsContext
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.isSpecified
import androidx.compose.ui.graphics.layer.GraphicsLayer
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.roundToIntSize
import co.typie.ui.theme.ResolvedThemeMode
import dev.chrisbanes.haze.HazeEffectDrawScope
import dev.chrisbanes.haze.HazeEffectFactory
import dev.chrisbanes.haze.HazeEffectLifecycleScope
import dev.chrisbanes.haze.HazeEffectRenderer
import dev.chrisbanes.haze.HazeEffectRendererDrawHooks
import dev.chrisbanes.haze.HazeEffectRendererLifecycle
import dev.chrisbanes.haze.HazeEffectRuntimeDrawScope
import dev.chrisbanes.haze.HazeSampling
import dev.chrisbanes.haze.InternalHazeApi
import dev.chrisbanes.haze.TrimMemoryLevel
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
@OptIn(InternalHazeApi::class)
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
) : HazeEffectFactory<Unit> {
  private var activeRenderer: Renderer? = null

  override fun createRenderer(): HazeEffectRenderer<Unit> = Renderer().also { activeRenderer = it }

  internal fun requestSample() {
    activeRenderer?.requestSample()
  }

  private inner class Renderer :
    HazeEffectRenderer<Unit>, HazeEffectRendererLifecycle<Unit>, HazeEffectRendererDrawHooks<Unit> {
    private var resolvedBackgroundColor = Color.Unspecified

    private var graphicsContext: GraphicsContext? = null
    private var contentLayer: GraphicsLayer? = null
    private var contentLayerSize: IntSize? = null
    private var analysisLayer: GraphicsLayer? = null
    private var sampleJob: Job? = null
    private var trailingSampleJob: Job? = null
    private var sampleRequestedWhileActive = false
    private var lastSampleStartedAt: TimeMark? = null
    private var lifecycleScope: HazeEffectLifecycleScope? = null

    override fun attach(scope: HazeEffectLifecycleScope) {
      lifecycleScope = scope
    }

    override fun update(scope: HazeEffectLifecycleScope, style: Unit, sampling: HazeSampling) {
      lifecycleScope = scope
      val currentBackgroundColor = backgroundColor()
      if (currentBackgroundColor != resolvedBackgroundColor) {
        resolvedBackgroundColor = currentBackgroundColor
        scope.invalidateDraw()
      }
    }

    override fun HazeEffectDrawScope.draw(style: Unit) {
      val scope = this as HazeEffectRuntimeDrawScope
      val layerSize = scope.layerSize.roundToIntSize()
      if (layerSize.width <= 0 || layerSize.height <= 0) return

      val layer = requireContentLayer(scope, layerSize)
      layer.record(size = layerSize) {
        val recordScope: DrawScope = this
        if (resolvedBackgroundColor.isSpecified) {
          drawRect(resolvedBackgroundColor)
        }

        with(scope) { recordScope.drawInput() }
      }

      if (samplingEnabled()) {
        scheduleSample(scope = scope, content = layer)
      }

      clipRect {
        val drawOffset = -scope.layerOffset
        translate(drawOffset.x, drawOffset.y) { drawLayer(layer) }
      }
    }

    override fun detach() {
      lifecycleScope = null
      releaseResources()
    }

    override fun onTrimMemory(level: TrimMemoryLevel) {
      releaseResources()
      lifecycleScope?.invalidateDraw()
    }

    override fun dispose() {
      releaseResources()
      if (activeRenderer === this) {
        activeRenderer = null
      }
    }

    override fun shouldClipToNodeBounds(): Boolean = true

    override fun shouldPreferClipToInputBounds(): Boolean =
      resolvedBackgroundColor.isSpecified && resolvedBackgroundColor.alpha < 0.9f

    fun requestSample() {
      lifecycleScope?.invalidateDraw()
    }

    private fun DrawScope.scheduleSample(
      scope: HazeEffectRuntimeDrawScope,
      content: GraphicsLayer,
    ) {
      if (sampleJob?.isActive == true) {
        sampleRequestedWhileActive = true
        return
      }
      val previousStart = lastSampleStartedAt
      if (previousStart != null) {
        val elapsed = previousStart.elapsedNow()
        if (elapsed < MINIMUM_LUMINANCE_SAMPLE_INTERVAL) {
          scheduleTrailingSample(MINIMUM_LUMINANCE_SAMPLE_INTERVAL - elapsed)
          return
        }
      }
      trailingSampleJob?.cancel()
      trailingSampleJob = null

      val region = resolveSampleRegion(scope) ?: return
      val token = sampleToken() ?: return

      val analysis = requireAnalysisLayer(scope)
      analysis.record(size = IntSize(LUMINANCE_ANALYSIS_WIDTH, LUMINANCE_ANALYSIS_HEIGHT)) {
        val scaleX = LUMINANCE_ANALYSIS_WIDTH / region.renderWidth
        val scaleY = LUMINANCE_ANALYSIS_HEIGHT / region.height
        scale(scaleX = scaleX, scaleY = scaleY, pivot = Offset.Zero) {
          translate(left = -scope.layerOffset.x, top = -(scope.layerOffset.y + region.top)) {
            drawLayer(content)
          }
        }
      }

      lastSampleStartedAt = TimeSource.Monotonic.markNow()
      sampleJob =
        scope.coroutineScope.launch {
          try {
            val reader = readPixels
            val sample =
              if (reader != null) {
                reader.invoke(analysis)
              } else {
                readAnalysisPixels(analysis)
              } ?: return@launch
            if (sampleToken() == token) {
              onPixels(token, sample.pixels, sample.width)
            }
          } finally {
            sampleJob = null
            if (sampleRequestedWhileActive && lifecycleScope != null) {
              sampleRequestedWhileActive = false
              scheduleTrailingSample()
            }
          }
        }
    }

    private fun resolveSampleRegion(scope: HazeEffectRuntimeDrawScope): LuminanceSampleRegion? {
      val density = scope.requireDensity()
      val renderWidth = scope.modifierSize.width.takeIf { it.isFinite() && it > 0f } ?: return null
      val renderHeight =
        scope.modifierSize.height.takeIf { it.isFinite() && it > 0f } ?: return null
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

    private fun scheduleTrailingSample(delayDuration: Duration = remainingSampleInterval()) {
      if (trailingSampleJob?.isActive == true) return
      val scope = lifecycleScope ?: return
      trailingSampleJob =
        scope.coroutineScope.launch {
          if (delayDuration.isPositive()) delay(delayDuration)
          trailingSampleJob = null
          if (lifecycleScope === scope) {
            scope.invalidateDraw()
          }
        }
    }

    private fun remainingSampleInterval(): Duration {
      val previousStart = lastSampleStartedAt ?: return Duration.ZERO
      return (MINIMUM_LUMINANCE_SAMPLE_INTERVAL - previousStart.elapsedNow()).coerceAtLeast(
        Duration.ZERO
      )
    }

    private fun requireContentLayer(
      scope: HazeEffectRuntimeDrawScope,
      size: IntSize,
    ): GraphicsLayer {
      val current = contentLayer
      if (current != null && !current.isReleased && contentLayerSize == size) {
        return current
      }

      releaseContentLayer()
      val currentGraphicsContext = scope.requireGraphicsContext()
      return currentGraphicsContext.createGraphicsLayer().also {
        graphicsContext = currentGraphicsContext
        contentLayer = it
        contentLayerSize = size
      }
    }

    private fun requireAnalysisLayer(scope: HazeEffectRuntimeDrawScope): GraphicsLayer {
      analysisLayer
        ?.takeUnless { it.isReleased }
        ?.let {
          return it
        }
      val currentGraphicsContext = graphicsContext ?: scope.requireGraphicsContext()
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
}
