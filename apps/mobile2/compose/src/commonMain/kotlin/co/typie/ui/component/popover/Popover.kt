package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.round
import androidx.compose.ui.window.Popup
import co.typie.ext.toDp
import co.typie.ext.toPx
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlin.time.TimeSource
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

data class PopoverPaneTransition(
  val progress: Float,
  val anchorContentRect: Rect,
)

val LocalPopoverPaneTransition = staticCompositionLocalOf<PopoverPaneTransition?> { null }

data class AnchorPointerState(
  val position: Offset,
  val isSelectionArmed: Boolean,
  val isUp: Boolean,
)

@Stable
class PopoverScope internal constructor(
  private val onClose: () -> Unit,
) {
  var pointerState: AnchorPointerState? by mutableStateOf(null)
    internal set

  fun close() {
    onClose()
  }
}

private enum class PopoverSlot {
  MeasurePane,
  Surface,
}

@Composable
fun Popover(
  anchor: @Composable () -> Unit,
  pane: @Composable PopoverScope.() -> Unit,
  position: PopoverPosition = PopoverPosition.BottomRight,
  maxWidth: Dp? = null,
  collapsedCornerRadius: Dp = 0.dp,
) {
  val density = LocalDensity.current
  val screenPaddingPx = PopoverDefaults.ScreenPadding.toPx(density).toInt()

  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  val animationProgress = remember { Animatable(0f) }

  val scope = remember {
    PopoverScope(onClose = { isExpanded = false })
  }

  LaunchedEffect(isExpanded) {
    if (isExpanded) {
      isOverlayVisible = true
      animationProgress.stop()
      animationProgress.snapTo(0f)
      animationProgress.animateTo(
        1f,
        tween(PopoverDefaults.ForwardDuration, easing = LinearEasing)
      )
    } else if (isOverlayVisible) {
      val from = if (animationProgress.value == 0f) 1f else animationProgress.value
      animationProgress.stop()
      animationProgress.snapTo(from)
      animationProgress.animateTo(
        0f,
        tween(PopoverDefaults.ReverseDuration, easing = LinearEasing)
      )
      isOverlayVisible = false
      scope.pointerState = null
    }
  }

  PlatformBackHandler(enabled = isOverlayVisible) {
    isExpanded = false
  }

  Box(
    modifier = Modifier
      .onGloballyPositioned { coordinates ->
        val pos = coordinates.positionInWindow().round()
        anchorBounds = IntRect(pos, coordinates.size)
      }
      .pointerInput(Unit) {
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent()
            if (event.type == PointerEventType.Press && !isOverlayVisible) {
              val press = event.changes.firstOrNull() ?: continue
              val anchorWindowOffset =
                Offset(anchorBounds.left.toFloat(), anchorBounds.top.toFloat())

              isExpanded = true
              scope.pointerState = AnchorPointerState(
                position = press.position + anchorWindowOffset,
                isSelectionArmed = false,
                isUp = false,
              )

              val armStartMark = TimeSource.Monotonic.markNow()
              val origin = press.position + anchorWindowOffset
              var isArmed = false

              while (true) {
                val moveEvent = awaitPointerEvent()
                val change = moveEvent.changes.find { it.id == press.id } ?: break

                val currentPos = change.position + anchorWindowOffset
                val elapsed = armStartMark.elapsedNow().inWholeMilliseconds
                val distance = (currentPos - origin).getDistance()

                if (!isArmed && elapsed >= PopoverDefaults.ArmDelayMs && distance > PopoverDefaults.ArmDistance) {
                  isArmed = true
                }

                scope.pointerState = AnchorPointerState(
                  position = currentPos,
                  isSelectionArmed = isArmed,
                  isUp = !change.pressed,
                )

                if (!change.pressed) break
              }
            }
          }
        }
      },
  ) {
    val progress = if (isOverlayVisible) {
      PopoverDefaults.PopoverEasing.transform(animationProgress.value).coerceIn(0f, 1f)
    } else {
      0f
    }

    Box(modifier = Modifier.alpha(1f - progress)) {
      anchor()
    }

    if (isOverlayVisible) {
      val positionProvider = remember(position, screenPaddingPx) {
        PopoverPositionProvider(position, screenPaddingPx)
      }

      Popup(
        popupPositionProvider = positionProvider,
        onDismissRequest = { isExpanded = false },
      ) {
        val effective = effectivePosition(position, positionProvider.lastShowBelow)

        PopoverPanePopup(
          anchor = anchor,
          pane = { scope.pane() },
          anchorSize = anchorBounds.size,
          effectivePosition = effective,
          progress = progress,
          collapsedCornerRadius = collapsedCornerRadius,
          maxWidth = maxWidth,
        )
      }
    }
  }
}

@Composable
private fun PopoverPanePopup(
  anchor: @Composable () -> Unit,
  pane: @Composable () -> Unit,
  anchorSize: IntSize,
  effectivePosition: PopoverPosition,
  progress: Float,
  collapsedCornerRadius: Dp,
  maxWidth: Dp?,
) {
  SubcomposeLayout(
    modifier = Modifier.then(if (maxWidth != null) Modifier.widthIn(max = maxWidth) else Modifier),
  ) { constraints ->
    val panePlaceables = subcompose(PopoverSlot.MeasurePane) {
      ShrinkWrappedPane(content = pane)
    }.map { it.measure(constraints.copy(minWidth = 0, minHeight = 0)) }

    val paneWidth = panePlaceables.maxOfOrNull { it.width } ?: anchorSize.width
    val paneHeight = panePlaceables.maxOfOrNull { it.height } ?: anchorSize.height
    val paneSize = IntSize(paneWidth, paneHeight)
    val transition = PopoverPaneTransition(
      progress = progress,
      anchorContentRect = anchorContentRect(paneSize, anchorSize, effectivePosition),
    )

    val surfacePlaceable = subcompose(PopoverSlot.Surface) {
      CompositionLocalProvider(LocalPopoverPaneTransition provides transition) {
        PopoverPaneSurface(
          anchor = anchor,
          pane = { ShrinkWrappedPane(content = pane) },
          paneSize = paneSize,
          anchorSize = anchorSize,
          effectivePosition = effectivePosition,
          progress = progress,
          collapsedCornerRadius = collapsedCornerRadius,
        )
      }
    }.single().measure(Constraints.fixed(paneWidth, paneHeight))

    layout(paneWidth, paneHeight) {
      surfacePlaceable.place(0, 0)
    }
  }
}

@Composable
private fun ShrinkWrappedPane(content: @Composable () -> Unit) {
  Box(modifier = Modifier.width(IntrinsicSize.Max)) {
    content()
  }
}

@Composable
private fun PopoverPaneSurface(
  anchor: @Composable () -> Unit,
  pane: @Composable () -> Unit,
  paneSize: IntSize,
  anchorSize: IntSize,
  effectivePosition: PopoverPosition,
  progress: Float,
  collapsedCornerRadius: Dp,
) {
  val density = LocalDensity.current
  val colors = AppTheme.colors
  val animatedWidth = sizeForProgress(anchorSize.width.toFloat(), paneSize.width.toFloat(), progress)
  val animatedHeight = sizeForProgress(anchorSize.height.toFloat(), paneSize.height.toFloat(), progress)
  val animatedSurfaceSize = IntSize(
    width = max(1, animatedWidth.roundToInt()),
    height = max(1, animatedHeight.roundToInt()),
  )
  val surfaceOffset = alignedOffset(paneSize, animatedSurfaceSize, effectivePosition)
  val paneOffset = alignedOffset(animatedSurfaceSize, paneSize, effectivePosition)
  val anchorOffset = anchorContentOffset(paneSize, anchorSize, effectivePosition)
  val cornerRadius = lerp(
    collapsedCornerRadius.toPx(density),
    PopoverDefaults.ExpandedRadius.toPx(density),
    progress,
  )
  val shape = SquircleShape(cornerRadius.toDp(density))
  val shadowElevation = (12f * progress).toDp(density)

  Box(
    modifier = Modifier.size(
      width = paneSize.width.toDp(density),
      height = paneSize.height.toDp(density),
    ),
  ) {
    Box(
      modifier = Modifier
        .offset { surfaceOffset }
        .size(
          width = animatedWidth.toDp(density),
          height = animatedHeight.toDp(density),
        )
        .shadow(shadowElevation, shape)
        .clip(shape)
        .background(colors.surfaceElevated, shape),
    ) {
      PopoverCropLayout(
        pane = {
          Box(modifier = Modifier.alpha(progress)) {
            pane()
          }
        },
        anchor = {
          Box(modifier = Modifier.alpha(1f - progress)) {
            anchor()
          }
        },
        paneSize = paneSize,
        anchorSize = anchorSize,
        paneOffset = paneOffset,
        anchorOffset = anchorOffset,
      )
    }
  }
}

@Composable
private fun PopoverCropLayout(
  pane: @Composable () -> Unit,
  anchor: @Composable () -> Unit,
  paneSize: IntSize,
  anchorSize: IntSize,
  paneOffset: IntOffset,
  anchorOffset: IntOffset,
) {
  Layout(
    content = {
      pane()
      anchor()
    },
    modifier = Modifier.fillMaxSize(),
  ) { measurables, constraints ->
    val panePlaceable = measurables[0].measure(Constraints.fixed(paneSize.width, paneSize.height))
    val anchorPlaceable = measurables[1].measure(Constraints.fixed(anchorSize.width, anchorSize.height))

    layout(constraints.maxWidth, constraints.maxHeight) {
      panePlaceable.place(paneOffset.x, paneOffset.y)
      anchorPlaceable.place(
        x = paneOffset.x + anchorOffset.x,
        y = paneOffset.y + anchorOffset.y,
      )
    }
  }
}

private fun lerp(start: Float, end: Float, fraction: Float): Float {
  return start + (end - start) * fraction
}

private fun sizeForProgress(start: Float, end: Float, progress: Float): Float {
  val size = lerp(start, end, progress)
  return if (start <= end) {
    max(start, size)
  } else {
    min(start, size)
  }
}

private fun anchorContentRect(
  paneSize: IntSize,
  anchorSize: IntSize,
  position: PopoverPosition,
): Rect {
  val offset = anchorContentOffset(paneSize, anchorSize, position)
  return Rect(
    left = offset.x.toFloat(),
    top = offset.y.toFloat(),
    right = offset.x + anchorSize.width.toFloat(),
    bottom = offset.y + anchorSize.height.toFloat(),
  )
}

private fun anchorContentOffset(
  paneSize: IntSize,
  anchorSize: IntSize,
  position: PopoverPosition,
): IntOffset {
  return alignedOffset(paneSize, anchorSize, position)
}

private fun alignedOffset(
  containerSize: IntSize,
  childSize: IntSize,
  position: PopoverPosition,
): IntOffset {
  val x = when (position) {
    PopoverPosition.BottomLeft, PopoverPosition.TopLeft -> 0
    PopoverPosition.BottomCenter, PopoverPosition.TopCenter -> (containerSize.width - childSize.width) / 2
    PopoverPosition.BottomRight, PopoverPosition.TopRight -> containerSize.width - childSize.width
  }
  val y = when (position) {
    PopoverPosition.BottomLeft, PopoverPosition.BottomCenter, PopoverPosition.BottomRight -> 0
    PopoverPosition.TopLeft, PopoverPosition.TopCenter, PopoverPosition.TopRight -> containerSize.height - childSize.height
  }

  return IntOffset(x, y)
}
