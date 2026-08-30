package co.typie.ui.component

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.GraphicsContext
import androidx.compose.ui.graphics.RenderEffect
import androidx.compose.ui.graphics.Shader
import androidx.compose.ui.graphics.ShaderBrush
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.isSpecified
import androidx.compose.ui.graphics.layer.GraphicsLayer
import androidx.compose.ui.graphics.layer.drawLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.roundToIntSize
import dev.chrisbanes.haze.HazeEffectDrawScope
import dev.chrisbanes.haze.HazeEffectFactory
import dev.chrisbanes.haze.HazeEffectLayoutScope
import dev.chrisbanes.haze.HazeEffectLifecycleScope
import dev.chrisbanes.haze.HazeEffectRenderer
import dev.chrisbanes.haze.HazeEffectRendererDrawHooks
import dev.chrisbanes.haze.HazeEffectRendererLifecycle
import dev.chrisbanes.haze.HazeEffectRuntimeDrawScope
import dev.chrisbanes.haze.HazeInput
import dev.chrisbanes.haze.HazeProgressive
import dev.chrisbanes.haze.HazeSampling
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.InternalHazeApi
import dev.chrisbanes.haze.PlatformRenderEffect
import dev.chrisbanes.haze.TrimMemoryLevel
import dev.chrisbanes.haze.asComposeRenderEffect
import dev.chrisbanes.haze.blur.HazeBlurStyle
import dev.chrisbanes.haze.blur.hazeBlur
import dev.chrisbanes.haze.createRuntimeEffect
import dev.chrisbanes.haze.createRuntimeShaderRenderEffect
import dev.chrisbanes.haze.hazeEffect
import dev.chrisbanes.haze.isRuntimeShaderRenderEffectSupported
import dev.chrisbanes.haze.then

/**
 * Applies the continuous progressive blur where runtime shaders exist, and falls back to Haze's
 * built-in progressive blur elsewhere.
 */
@Composable
@OptIn(InternalHazeApi::class)
internal fun typieProgressiveBlur(
  hazeState: HazeState,
  radius: Dp,
  progressiveBrush: Brush,
  fallbackProgressive: HazeProgressive,
  backdropColor: () -> Color,
): Modifier {
  val input = remember(hazeState) { HazeInput.Sources(hazeState) }

  if (!isRuntimeShaderRenderEffectSupported()) {
    val resolvedBackdropColor = backdropColor()
    return Modifier.hazeBlur(
      input = input,
      style =
        HazeBlurStyle {
          blurRadius(radius)
          backgroundColor(resolvedBackdropColor)
          progressive(fallbackProgressive)
        },
    )
  }

  val currentBackdropColor = rememberUpdatedState(backdropColor)
  val factory =
    remember(radius, progressiveBrush) {
      TypieProgressiveBlurEffect(
        blurRadius = radius,
        progressiveBrush = progressiveBrush,
        backgroundColor = { currentBackdropColor.value() },
      )
    }

  return Modifier.hazeEffect(factory = factory, input = input, style = Unit)
}

/**
 * Workaround for Haze's quantized progressive blur radius.
 *
 * Remove this effect when the adopted Haze version provides continuous fractional-radius
 * progressive blur and the focused pixel regressions pass with its built-in effect.
 */
@Stable
@OptIn(InternalHazeApi::class)
internal class TypieProgressiveBlurEffect(
  private val blurRadius: Dp,
  private val progressiveBrush: Brush,
  private val backgroundColor: () -> Color,
) : HazeEffectFactory<Unit> {
  override fun createRenderer(): HazeEffectRenderer<Unit> = Renderer()

  private inner class Renderer :
    HazeEffectRenderer<Unit>, HazeEffectRendererLifecycle<Unit>, HazeEffectRendererDrawHooks<Unit> {
    private var resolvedBackgroundColor: Color = Color.Unspecified

    private var graphicsContext: GraphicsContext? = null
    private var contentLayer: GraphicsLayer? = null
    private var contentLayerSize: IntSize? = null
    private var renderEffect: RenderEffect? = null
    private var renderEffectKey: RenderEffectKey? = null

    override fun update(scope: HazeEffectLifecycleScope, style: Unit, sampling: HazeSampling) {
      val currentBackgroundColor = backgroundColor()
      if (currentBackgroundColor != resolvedBackgroundColor) {
        resolvedBackgroundColor = currentBackgroundColor
        scope.invalidateDraw()
      }
    }

    override fun HazeEffectDrawScope.draw(style: Unit) {
      drawRuntimeEffect(this as HazeEffectRuntimeDrawScope)
    }

    override fun detach() {
      releaseRuntimeResources()
    }

    override fun onTrimMemory(level: TrimMemoryLevel) {
      releaseRuntimeResources()
    }

    override fun dispose() {
      releaseRuntimeResources()
    }

    override fun shouldClipToNodeBounds(): Boolean = true

    override fun shouldPreferClipToInputBounds(): Boolean =
      resolvedBackgroundColor.isSpecified && resolvedBackgroundColor.alpha < 0.9f

    override fun HazeEffectLayoutScope.calculateLayerBounds(style: Unit): Rect {
      val blurRadiusPx = blurRadius.toPx()
      return if (blurRadiusPx >= 1f) modifierBounds.inflate(blurRadiusPx) else modifierBounds
    }

    private fun DrawScope.drawRuntimeEffect(scope: HazeEffectRuntimeDrawScope) {
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

      layer.clip = true
      layer.renderEffect = getOrCreateRenderEffect(scope)

      clipRect {
        val drawOffset = -scope.layerOffset
        translate(drawOffset.x, drawOffset.y) { drawLayer(layer) }
      }
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

    private fun getOrCreateRenderEffect(scope: HazeEffectRuntimeDrawScope): RenderEffect {
      val blurRadiusPx = with(scope.requireDensity()) { blurRadius.toPx() }
      val renderSize = scope.modifierSize.takeIf { it.isSpecified } ?: Size.Zero
      val key =
        RenderEffectKey(
          blurRadiusPx = blurRadiusPx,
          layerSize = scope.layerSize,
          layerOffset = scope.layerOffset,
          renderSize = renderSize,
        )
      renderEffect
        ?.takeIf { renderEffectKey == key }
        ?.let {
          return it
        }

      val mask = progressiveBrush.toShader(renderSize)
      return createProgressiveBlurRenderEffect(
          blurRadiusPx = blurRadiusPx,
          size = renderSize,
          offset = scope.layerOffset,
          mask = mask,
        )
        .asComposeRenderEffect()
        .also {
          renderEffect = it
          renderEffectKey = key
        }
    }

    private fun releaseRuntimeResources() {
      releaseContentLayer()
      renderEffect = null
      renderEffectKey = null
    }

    private fun releaseContentLayer() {
      val layer = contentLayer
      val context = graphicsContext
      if (layer != null && context != null && !layer.isReleased) {
        context.releaseGraphicsLayer(layer)
      }
      contentLayer = null
      contentLayerSize = null
      graphicsContext = null
    }
  }
}

private data class RenderEffectKey(
  val blurRadiusPx: Float,
  val layerSize: Size,
  val layerOffset: Offset,
  val renderSize: Size,
)

@OptIn(InternalHazeApi::class)
private fun createProgressiveBlurRenderEffect(
  blurRadiusPx: Float,
  size: Size,
  offset: Offset,
  mask: Shader,
): PlatformRenderEffect {
  fun shader(vertical: Boolean): PlatformRenderEffect =
    createRuntimeShaderRenderEffect(
      effect =
        if (vertical) {
          VerticalProgressiveBlurRuntimeEffect
        } else {
          HorizontalProgressiveBlurRuntimeEffect
        },
      shaderNames = arrayOf("content"),
      inputs = arrayOf<PlatformRenderEffect?>(null),
    ) {
      setFloatUniform("blurRadius", blurRadiusPx)
      setFloatUniform("crop", offset.x, offset.y, offset.x + size.width, offset.y + size.height)
      setChildShader("mask", mask)
    }

  return shader(vertical = false).then(shader(vertical = true))
}

private fun Brush.toShader(size: Size): Shader =
  requireNotNull((this as? ShaderBrush)?.createShader(size)) {
    "Progressive blur requires a ShaderBrush"
  }

// Adapted from Haze 2.0.0-alpha03's HazeBlurShaders under Apache-2.0.
// Copyright 2024, Christopher Banes and the Haze project contributors.
@OptIn(InternalHazeApi::class)
private val VerticalProgressiveBlurRuntimeEffect by
  lazy(LazyThreadSafetyMode.NONE) {
    createRuntimeEffect(progressiveBlurShaderSource(vertical = true))
  }

@OptIn(InternalHazeApi::class)
private val HorizontalProgressiveBlurRuntimeEffect by
  lazy(LazyThreadSafetyMode.NONE) {
    createRuntimeEffect(progressiveBlurShaderSource(vertical = false))
  }

private fun progressiveBlurShaderSource(vertical: Boolean): String =
  """
  uniform shader content;
  uniform float blurRadius;
  uniform vec4 crop;
  uniform shader mask;

  const half maxRadius = 150.0;

  float gaussian(float x, float sigma) {
    return exp(-(x * x) / (2.0 * sigma * sigma));
  }

  float tapCoverage(float sampleOffset, float radius) {
    return clamp(radius - sampleOffset + 1.0, 0.0, 1.0);
  }

  vec4 blur(vec2 coord, float radius) {
    float sigma = max(radius / 2.0, 1.0);
    float weightSum = 1.0;
    vec4 result = content.eval(coord);

    for (half i = 1.0; i < maxRadius; i += 2.0) {
      float weightL = gaussian(i, sigma) * tapCoverage(i, radius);
      float weightH = gaussian(i + 1.0, sigma) * tapCoverage(i + 1.0, radius);
      float weight = weightL + weightH;
      if (weight <= 0.0) { break; }

      vec2 sampleOffset =
        ${if (vertical) "vec2(0.0, i + weightH / weight)" else "vec2(i + weightH / weight, 0.0)"};

      vec2 newCoord = coord - sampleOffset;
      if (newCoord.x >= crop[0] && newCoord.y >= crop[1]) {
        result += weight * content.eval(newCoord);
        weightSum += weight;
      }

      newCoord = coord + sampleOffset;
      if (newCoord.x <= crop[2] && newCoord.y <= crop[3]) {
        result += weight * content.eval(newCoord);
        weightSum += weight;
      }
    }

    return result / weightSum;
  }

  vec4 main(vec2 coord) {
    vec2 maskCoord = max(coord - crop.xy, vec2(0.0, 0.0));
    float intensity = mask.eval(maskCoord).a;

    return blur(coord, mix(0.0, blurRadius, intensity));
  }
  """
