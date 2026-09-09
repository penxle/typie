package co.typie.ui.component.tooltip

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.VectorConverter
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.drawOutline
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.graphics.drawscope.inset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.toSize
import co.typie.ext.safeDrawing
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import co.typie.ui.theme.shadow
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch

private val TooltipMotionEasing = CubicBezierEasing(0.2f, 0f, 0f, 1f)
private val TooltipOutroEasing = CubicBezierEasing(0.4f, 0f, 1f, 1f)
private val TooltipCrossfadeEasing = CubicBezierEasing(0f, 0f, 0.58f, 1f)

private data class TooltipPresentation(
  val anchor: TooltipAnchor,
  val bounds: Rect,
  val content: TooltipContent,
  val size: IntSize,
  val position: TooltipPosition,
)

@Composable
internal fun TooltipHost(state: TooltipState) {
  val windowInfo = LocalWindowInfo.current
  LaunchedEffect(windowInfo.isWindowFocused) { if (!windowInfo.isWindowFocused) state.dismiss() }
  var origin by remember { mutableStateOf(Offset.Zero) }
  var viewport by remember { mutableStateOf(IntSize.Zero) }
  val active = state.active

  // Layer translations need frame tracking. Use unclipped bounds for placement, but close
  // when the reference is fully hidden by a clipping ancestor or the safe viewport.
  LaunchedEffect(active, origin, viewport) {
    val anchor = active ?: return@LaunchedEffect
    while (true) {
      withFrameNanos {
        anchor.coordinates?.let { coordinates ->
          if (
            !coordinates.isAttached ||
              coordinates.boundsInWindow().intersect(Rect(origin, viewport.toSize())).isEmpty
          ) {
            state.remove(anchor)
          } else {
            anchor.bounds = coordinates.boundsInWindow(clipBounds = false)
          }
        }
      }
    }
  }

  Box(
    Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).onGloballyPositioned {
      origin = it.positionInWindow()
      viewport = it.size
    }
  ) {
    if (viewport == IntSize.Zero) return@Box
    TooltipBubble(state, active, origin, viewport)
  }
}

@Composable
private fun TooltipBubble(
  state: TooltipState,
  active: TooltipAnchor?,
  origin: Offset,
  viewport: IntSize,
) {
  val density = LocalDensity.current
  val gap = with(density) { 8.dp.toPx() }
  val paddingX = with(density) { 8.dp.roundToPx() }
  val paddingY = with(density) { 4.dp.roundToPx() }
  val arrowExtent = with(density) { 6.dp.roundToPx() }
  val maxTravel = with(density) { 160.dp.toPx() }
  val content =
    rememberTooltipContent(
      active?.text.orEmpty(),
      active?.shortcut,
      (viewport.width - (gap * 2).roundToInt() - paddingX * 2).coerceAtLeast(1),
    )
  val target =
    active
      ?.takeUnless { it.bounds.isEmpty }
      ?.let { anchor ->
        val bounds = anchor.bounds.translate(-origin)
        val size = IntSize(content.size.width + paddingX * 2, content.size.height + paddingY * 2)
        TooltipPresentation(
          anchor,
          bounds,
          content,
          size,
          tooltipPosition(bounds, viewport, size, gap, anchor.placement),
        )
      }
  var displayed by remember { mutableStateOf<TooltipPresentation?>(null) }
  var outgoing by remember { mutableStateOf<TooltipContent?>(null) }
  val position = remember { Animatable(IntOffset.Zero, IntOffset.VectorConverter) }
  val bodySize = remember { Animatable(IntSize.Zero, IntSize.VectorConverter) }
  val presence = remember { Animatable(0f) }
  val transferOpacity = remember { Animatable(1f) }
  val contentOpacity = remember { Animatable(1f) }

  LaunchedEffect(target != null) {
    if (target != null) {
      presence.animateTo(
        1f,
        tween(if (presence.value == 0f) 200 else 100, easing = TooltipMotionEasing),
      )
    } else {
      presence.animateTo(0f, tween(100, easing = TooltipOutroEasing))
      displayed = null
      outgoing = null
      state.onHidden()
    }
  }

  // Only target/content changes own a finite transfer animation. Live movement of the same
  // reference cancels that animation and snaps to its actual position, including during travel.
  LaunchedEffect(target) {
    val next = target ?: return@LaunchedEffect
    val previous = displayed
    if (previous != null && previous.anchor !== next.anchor && presence.value < 1f)
      presence.snapTo(1f)
    val referenceMoved = previous?.anchor === next.anchor && previous.bounds != next.bounds
    val travels =
      previous != null &&
        previous.position.above == next.position.above &&
        (Offset(position.value.x.toFloat(), position.value.y.toFloat()) -
            Offset(next.position.offset.x.toFloat(), next.position.offset.y.toFloat()))
          .getDistance() <= maxTravel
    if (previous == null || referenceMoved) {
      displayed = next
      outgoing = null
      position.snapTo(next.position.offset)
      bodySize.snapTo(next.size)
      contentOpacity.snapTo(1f)
      transferOpacity.snapTo(1f)
    } else if (travels) {
      outgoing = previous.content.takeUnless { it == next.content }
      displayed = next
      transferOpacity.snapTo(1f)
      contentOpacity.snapTo(if (outgoing == null) 1f else 0f)
      coroutineScope {
        launch {
          position.animateTo(next.position.offset, tween(120, easing = TooltipMotionEasing))
        }
        launch { bodySize.animateTo(next.size, tween(120, easing = TooltipMotionEasing)) }
        launch { contentOpacity.animateTo(1f, tween(120, easing = LinearEasing)) }
      }
      outgoing = null
    } else {
      transferOpacity.animateTo(0f, tween(40, easing = TooltipCrossfadeEasing))
      displayed = next
      outgoing = null
      position.snapTo(next.position.offset)
      bodySize.snapTo(next.size)
      contentOpacity.snapTo(1f)
      transferOpacity.animateTo(1f, tween(40, easing = TooltipCrossfadeEasing))
    }
  }

  val presentation = displayed ?: return
  val isDark = AppTheme.themeMode == ResolvedThemeMode.Dark
  val background = if (isDark) AppTheme.colors.surfaceActive else AppTheme.colors.surfaceInverse
  val foreground = if (isDark) AppTheme.colors.textDefault else AppTheme.colors.textOnInverse
  val shadow = AppTheme.shadows.md
  val shadowPadding =
    remember(shadow) {
      shadow.layers.maxOfOrNull {
        it.blur + it.spread + maxOf(abs(it.offsetX.value), abs(it.offsetY.value)).dp
      } ?: 0.dp
    }
  val shadowPaddingPx = with(density) { shadowPadding.roundToPx() }
  val shape =
    TooltipShape(
      presentation.bounds.center.x - position.value.x,
      presentation.position.above,
      arrowExtent.toFloat(),
    )
  // Keep the shadow inside the animated layer's actual bounds, including during opacity changes.
  Box(
    Modifier.offset { position.value - IntOffset(shadowPaddingPx, arrowExtent + shadowPaddingPx) }
      .wrapContentSize(Alignment.TopStart, unbounded = true)
      .graphicsLayer {
        alpha = presence.value * transferOpacity.value
        scaleX = 0.9f + presence.value * 0.1f
        scaleY = scaleX
      }
      .padding(shadowPadding)
      .clearAndSetSemantics {}
  ) {
    Canvas(
      Modifier.size(
          with(density) { bodySize.value.width.toDp() },
          with(density) { (bodySize.value.height + arrowExtent * 2).toDp() },
        )
        .shadow(shadow, shape)
    ) {
      drawOutline(shape.createOutline(size, layoutDirection, this), background)
      inset(left = 0f, top = arrowExtent.toFloat(), right = 0f, bottom = arrowExtent.toFloat()) {
        clipRect {
          outgoing?.let { drawTooltipContent(it, foreground, 1f - contentOpacity.value) }
          drawTooltipContent(presentation.content, foreground, contentOpacity.value)
        }
      }
    }
  }
}

internal data class TooltipPosition(val offset: IntOffset, val above: Boolean)

internal fun tooltipPosition(
  anchor: Rect,
  viewport: IntSize,
  bubble: IntSize,
  gap: Float,
  preferred: TooltipPlacement = TooltipPlacement.Below,
): TooltipPosition {
  val fitsAbove = anchor.top - gap - bubble.height >= gap
  val fitsBelow = anchor.bottom + gap + bubble.height <= viewport.height - gap
  val above =
    when {
      preferred == TooltipPlacement.Above && fitsAbove -> true
      preferred == TooltipPlacement.Below && fitsBelow -> false
      fitsAbove -> true
      fitsBelow -> false
      else -> anchor.center.y > viewport.height / 2f
    }
  val maxX = (viewport.width - bubble.width - gap).coerceAtLeast(gap)
  val maxY = (viewport.height - bubble.height - gap).coerceAtLeast(gap)
  return TooltipPosition(
    offset =
      IntOffset(
        (anchor.center.x - bubble.width / 2f).coerceIn(gap, maxX).roundToInt(),
        (if (above) anchor.top - gap - bubble.height else anchor.bottom + gap)
          .coerceIn(gap, maxY)
          .roundToInt(),
      ),
    above = above,
  )
}
