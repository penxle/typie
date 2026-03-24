package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
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
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.round
import androidx.compose.ui.window.Popup
import co.typie.ext.EdgeAutoScrollState
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.edgeAutoScroll
import co.typie.ext.overscroll
import co.typie.ext.rememberEdgeAutoScrollState
import co.typie.ext.safeDrawing
import co.typie.ext.toDp
import co.typie.ext.toPx
import co.typie.ext.verticalScroll
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.time.TimeSource

data class PopoverPaneTransition(
  val progress: Float,
  val anchorContentRect: Rect,
)

val LocalPopoverPaneTransition = staticCompositionLocalOf<PopoverPaneTransition?> { null }
val LocalPopoverPaneEdgeAutoScrollState = staticCompositionLocalOf<EdgeAutoScrollState?> { null }

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
  InitialMeasurePane,
  FinalMeasurePane,
  Surface,
}

internal data class PopoverScreenPadding(
  val left: Int,
  val top: Int,
  val right: Int,
  val bottom: Int,
)

@Composable
fun Popover(
  anchor: @Composable () -> Unit,
  pane: @Composable PopoverScope.() -> Unit,
  position: PopoverPosition = PopoverPosition.BottomRight,
  maxWidth: Dp? = null,
  collapsedCornerRadius: Dp = 0.dp,
) {
  val density = LocalDensity.current
  val layoutDirection = LocalLayoutDirection.current
  val screenPaddingPx = PopoverDefaults.ScreenPadding.toPx(density).toInt()
  val armDistancePx = PopoverDefaults.ArmDistance.toPx(density)
  val safeDrawing = WindowInsets.safeDrawing
  val screenPadding = PopoverScreenPadding(
    left = screenPaddingPx + safeDrawing.getLeft(density, layoutDirection),
    top = screenPaddingPx + safeDrawing.getTop(density),
    right = screenPaddingPx + safeDrawing.getRight(density, layoutDirection),
    bottom = screenPaddingPx + safeDrawing.getBottom(density),
  )

  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  val animationProgress = remember { Animatable(0f) }
  val scrollGestureLockState = LocalScrollGestureLockState.current

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
              val origin = press.position + anchorWindowOffset
              val gestureTracker = PopoverAnchorGestureTracker(
                origin = origin,
                armDistancePx = armDistancePx,
              )
              val scrollLockHandle = scrollGestureLockState.acquire()

              try {
                isExpanded = true
                val startUpdate = gestureTracker.start()
                scope.pointerState = startUpdate.pointerState
                if (startUpdate.consumeChange) {
                  // Anchor-originated scrub should own the gesture so parent scroll containers stay still.
                  press.consume()
                }

                val armStartMark = TimeSource.Monotonic.markNow()

                while (true) {
                  val moveEvent = awaitPointerEvent()
                  val change = moveEvent.changes.find { it.id == press.id } ?: break

                  val currentPos = change.position + anchorWindowOffset
                  val elapsed = armStartMark.elapsedNow().inWholeMilliseconds
                  val update = gestureTracker.update(
                    currentPosition = currentPos,
                    elapsedMillis = elapsed,
                    isPressed = change.pressed,
                  )
                  scope.pointerState = update.pointerState
                  if (update.consumeChange) {
                    change.consume()
                  }

                  if (!change.pressed) break
                }
              } finally {
                scrollLockHandle.release()
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
      val positionProvider = remember(position, screenPadding) {
        PopoverPositionProvider(position, screenPadding)
      }

      Popup(
        popupPositionProvider = positionProvider,
        onDismissRequest = { isExpanded = false },
      ) {
        PopoverPanePopup(
          anchor = anchor,
          pane = { scope.pane() },
          anchorBounds = anchorBounds,
          position = position,
          progress = progress,
          collapsedCornerRadius = collapsedCornerRadius,
          screenPadding = screenPadding,
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
  anchorBounds: IntRect,
  position: PopoverPosition,
  progress: Float,
  collapsedCornerRadius: Dp,
  screenPadding: PopoverScreenPadding,
  maxWidth: Dp?,
) {
  SubcomposeLayout(
    modifier = Modifier.then(if (maxWidth != null) Modifier.widthIn(max = maxWidth) else Modifier),
  ) { constraints ->
    val paneConstraints = constraints.copy(
      minWidth = 0,
      minHeight = 0,
      maxWidth = shrinkBounded(constraints.maxWidth, screenPadding.left + screenPadding.right),
      maxHeight = shrinkBounded(constraints.maxHeight, screenPadding.top + screenPadding.bottom),
    )

    val initialPanePlaceables = subcompose(PopoverSlot.InitialMeasurePane) {
      ShrinkWrappedPane(content = pane)
    }.map { it.measure(paneConstraints) }

    val initiallyMeasuredWidth =
      initialPanePlaceables.maxOfOrNull { it.width } ?: anchorBounds.width
    val initiallyMeasuredHeight =
      initialPanePlaceables.maxOfOrNull { it.height } ?: anchorBounds.height
    val showBelow = shouldShowBelow(
      position = position,
      childHeight = initiallyMeasuredHeight,
      windowHeight = constraints.maxHeight,
      anchorRect = anchorBounds,
      screenPadding = screenPadding,
    )
    val finalPaneConstraints = paneConstraints.copy(
      maxHeight = availableHeightForPlacement(
        windowHeight = constraints.maxHeight,
        anchorBounds = anchorBounds,
        screenPadding = screenPadding,
        showBelow = showBelow,
      ),
    )
    val finalPanePlaceables = subcompose(PopoverSlot.FinalMeasurePane) {
      ShrinkWrappedPane(content = pane)
    }.map { it.measure(finalPaneConstraints) }

    val paneWidth = finalPanePlaceables.maxOfOrNull { it.width } ?: initiallyMeasuredWidth
    val paneHeight = finalPanePlaceables.maxOfOrNull { it.height } ?: initiallyMeasuredHeight
    val paneSize = IntSize(paneWidth, paneHeight)
    val anchorSize = anchorBounds.size
    val effectivePosition = effectivePosition(position, showBelow)
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
  val scrollState = rememberScrollState()
  val edgeAutoScrollState = rememberEdgeAutoScrollState(verticalScrollableState = scrollState)

  CompositionLocalProvider(
    LocalPopoverPaneEdgeAutoScrollState provides edgeAutoScrollState,
  ) {
    Box(
      modifier = Modifier
        .width(IntrinsicSize.Max)
        .edgeAutoScroll(edgeAutoScrollState)
        .verticalScroll(scrollState)
        .overscroll(),
    ) {
      content()
    }
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
  val animatedWidth =
    sizeForProgress(anchorSize.width.toFloat(), paneSize.width.toFloat(), progress)
  val animatedHeight =
    sizeForProgress(anchorSize.height.toFloat(), paneSize.height.toFloat(), progress)
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
  val shadowElevation = (12f * progress).dp

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
        .background(AppTheme.colors.surfaceRaised, shape),
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
    val anchorPlaceable =
      measurables[1].measure(Constraints.fixed(anchorSize.width, anchorSize.height))

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

private fun shrinkBounded(value: Int, inset: Int): Int {
  if (value == Constraints.Infinity) {
    return value
  }

  return max(0, value - inset)
}

private fun availableHeightForPlacement(
  windowHeight: Int,
  anchorBounds: IntRect,
  screenPadding: PopoverScreenPadding,
  showBelow: Boolean,
): Int {
  if (windowHeight == Constraints.Infinity) {
    return windowHeight
  }

  return if (showBelow) {
    max(0, windowHeight - screenPadding.bottom - anchorBounds.top)
  } else {
    max(0, anchorBounds.bottom - screenPadding.top)
  }
}
