package co.typie.navigation

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.geometry.isSpecified
import androidx.compose.ui.geometry.takeOrElse
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
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.roundToIntSize
import dev.chrisbanes.haze.ExperimentalHazeApi
import dev.chrisbanes.haze.InternalHazeApi
import dev.chrisbanes.haze.PlatformRenderEffect
import dev.chrisbanes.haze.TrimMemoryLevel
import dev.chrisbanes.haze.VisualEffect
import dev.chrisbanes.haze.VisualEffectContext
import dev.chrisbanes.haze.asComposeRenderEffect
import dev.chrisbanes.haze.blur.BlurVisualEffect
import dev.chrisbanes.haze.blur.HazeProgressive
import dev.chrisbanes.haze.createRuntimeEffect
import dev.chrisbanes.haze.createRuntimeShaderRenderEffect
import dev.chrisbanes.haze.isRuntimeShaderRenderEffectSupported
import dev.chrisbanes.haze.then

/**
 * Top-bar-only workaround for Haze's quantized progressive blur radius.
 *
 * Remove this effect when the adopted Haze version provides continuous fractional-radius
 * progressive blur and the Desktop regression passes with its built-in effect.
 */
@Stable
@OptIn(ExperimentalHazeApi::class, InternalHazeApi::class)
internal class TypieTopBarProgressiveBlurEffect(
  private val blurRadius: Dp,
  private val progressiveBrush: Brush,
  fallbackProgressive: HazeProgressive,
  backgroundColor: Color,
) : VisualEffect {
  var backgroundColor: Color by mutableStateOf(backgroundColor)

  private var resolvedBackgroundColor: Color = backgroundColor
  private val fallbackEffect =
    BlurVisualEffect().apply {
      this.blurRadius = blurRadius
      this.backgroundColor = backgroundColor
      progressive = fallbackProgressive
    }

  private var graphicsContext: GraphicsContext? = null
  private var contentLayer: GraphicsLayer? = null
  private var contentLayerSize: IntSize? = null
  private var renderEffect: RenderEffect? = null
  private var renderEffectKey: RenderEffectKey? = null

  override fun attach(context: VisualEffectContext) {
    fallbackEffect.attach(context)
  }

  override fun update(context: VisualEffectContext) {
    val currentBackgroundColor = backgroundColor
    if (currentBackgroundColor != resolvedBackgroundColor) {
      resolvedBackgroundColor = currentBackgroundColor
      fallbackEffect.backgroundColor = currentBackgroundColor
      context.invalidateDraw()
    }
    fallbackEffect.update(context)
  }

  override fun DrawScope.draw(context: VisualEffectContext) {
    if (!isRuntimeShaderRenderEffectSupported()) {
      with(fallbackEffect) { draw(context) }
      return
    }

    drawRuntimeEffect(context)
  }

  override fun detach(context: VisualEffectContext) {
    releaseRuntimeResources()
    fallbackEffect.detach(context)
  }

  override fun onTrimMemory(context: VisualEffectContext, level: TrimMemoryLevel) {
    releaseRuntimeResources()
    fallbackEffect.onTrimMemory(context, level)
    context.invalidateDraw()
  }

  override fun shouldDrawContentBehind(context: VisualEffectContext): Boolean =
    fallbackEffect.shouldDrawContentBehind(context)

  override fun shouldClipToNodeBounds(): Boolean = true

  override fun shouldPreferClipToAreaBounds(): Boolean =
    resolvedBackgroundColor.isSpecified && resolvedBackgroundColor.alpha < 0.9f

  override fun calculateLayerBounds(rect: Rect, density: Density): Rect {
    val blurRadiusPx = with(density) { blurRadius.toPx() }
    return if (blurRadiusPx >= 1f) rect.inflate(blurRadiusPx) else rect
  }

  private fun DrawScope.drawRuntimeEffect(context: VisualEffectContext) {
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

    layer.clip = true
    layer.renderEffect = getOrCreateRenderEffect(context)

    clipRect {
      val drawOffset = -context.layerOffset
      translate(drawOffset.x, drawOffset.y) { drawLayer(layer) }
    }
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

  private fun getOrCreateRenderEffect(context: VisualEffectContext): RenderEffect {
    val blurRadiusPx = with(context.requireDensity()) { blurRadius.toPx() }
    val renderSize = context.size.takeIf { it.isSpecified } ?: Size.Zero
    val key =
      RenderEffectKey(
        blurRadiusPx = blurRadiusPx,
        layerSize = context.layerSize,
        layerOffset = context.layerOffset,
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
        offset = context.layerOffset,
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
    "Top bar progressive blur requires a ShaderBrush"
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
