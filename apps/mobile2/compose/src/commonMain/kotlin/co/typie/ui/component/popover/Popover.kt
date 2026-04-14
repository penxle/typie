package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.pointer.PointerEventPass
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
import androidx.compose.ui.window.PopupProperties
import co.typie.ext.EdgeAutoScrollState
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.edgeAutoScroll
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

data class PopoverPaneTransition(val progress: Float, val anchorContentRect: Rect)

val LocalPopoverPaneTransition = staticCompositionLocalOf<PopoverPaneTransition?> { null }
val LocalPopoverPaneEdgeAutoScrollState = staticCompositionLocalOf<EdgeAutoScrollState?> { null }

data class AnchorPointerState(
  val position: Offset,
  val isSelectionArmed: Boolean,
  val isUp: Boolean,
)

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
  pane:
    @Composable
    context(PopoverScope)
    () -> Unit,
  placement: PopoverPlacement = PopoverPlacement.BelowEnd,
  maxWidth: Dp? = null,
  minWidth: Dp = 0.dp,
  expandToMaxWidth: Boolean = false,
  screenPadding: PaddingValues = PaddingValues(all = PopoverDefaults.ScreenPadding),
  collapsedCornerRadius: Dp = 0.dp,
) {
  val density = LocalDensity.current
  val layoutDirection = LocalLayoutDirection.current
  val armDistancePx = PopoverDefaults.ArmDistance.toPx(density)
  val safeDrawing = WindowInsets.safeDrawing
  val resolvedScreenPadding =
    PopoverScreenPadding(
      left =
        screenPadding.calculateStartPadding(layoutDirection).toPx(density).toInt() +
          safeDrawing.getLeft(density, layoutDirection),
      top = screenPadding.calculateTopPadding().toPx(density).toInt() + safeDrawing.getTop(density),
      right =
        screenPadding.calculateEndPadding(layoutDirection).toPx(density).toInt() +
          safeDrawing.getRight(density, layoutDirection),
      bottom =
        screenPadding.calculateBottomPadding().toPx(density).toInt() +
          safeDrawing.getBottom(density),
    )

  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  var paneBoundsInWindow by remember { mutableStateOf<Rect?>(null) }
  var paneSizeInWindow by remember { mutableStateOf<IntSize?>(null) }
  var outsideDismissGestureActive by remember { mutableStateOf(false) }
  var reverseAnimationCompleted by remember { mutableStateOf(false) }
  val animationProgress = remember { Animatable(0f) }
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val outsideTapHostState = LocalPopoverOutsideTapHostState.current
  val scope = remember { PopoverScope(onClose = { isExpanded = false }) }
  val dismissPopoverFromOutsideGesture by rememberUpdatedState {
    outsideDismissGestureActive = true
    scope.close()
  }
  val finishOutsideDismissGesture by rememberUpdatedState { outsideDismissGestureActive = false }
  var outsideTapHostHandle by remember { mutableStateOf<PopoverOutsideTapHostHandle?>(null) }

  LaunchedEffect(isExpanded) {
    if (isExpanded) {
      outsideDismissGestureActive = false
      reverseAnimationCompleted = false
      scope.acceptsInput = true
      isOverlayVisible = true
      animationProgress.stop()
      animationProgress.snapTo(0f)
      animationProgress.animateTo(1f, tween(PopoverDefaults.ForwardDuration, easing = LinearEasing))
    } else if (isOverlayVisible) {
      val from = if (animationProgress.value == 0f) 1f else animationProgress.value
      reverseAnimationCompleted = false
      animationProgress.stop()
      animationProgress.snapTo(from)
      animationProgress.animateTo(0f, tween(PopoverDefaults.ReverseDuration, easing = LinearEasing))
      reverseAnimationCompleted = true
    }
  }

  LaunchedEffect(
    isExpanded,
    isOverlayVisible,
    reverseAnimationCompleted,
    outsideDismissGestureActive,
  ) {
    if (
      !isExpanded && isOverlayVisible && reverseAnimationCompleted && !outsideDismissGestureActive
    ) {
      isOverlayVisible = false
      paneBoundsInWindow = null
      paneSizeInWindow = null
      scope.pointerState = null
      reverseAnimationCompleted = false
    }
  }

  DisposableEffect(outsideTapHostState, isExpanded) {
    if (outsideTapHostState == null || !isExpanded) {
      outsideTapHostHandle = null
      onDispose {}
    } else {
      val handle = outsideTapHostState.register()
      outsideTapHostHandle = handle
      onDispose {
        handle.clear()
        if (outsideTapHostHandle === handle) {
          outsideTapHostHandle = null
        }
      }
    }
  }

  SideEffect {
    if (isExpanded) {
      outsideTapHostHandle?.update(
        paneBounds = paneBoundsInWindow,
        onDismiss = dismissPopoverFromOutsideGesture,
        onDismissGestureFinished = finishOutsideDismissGesture,
      )
    }
  }

  PlatformBackHandler(enabled = isOverlayVisible) { isExpanded = false }

  Box(
    modifier =
      Modifier.onGloballyPositioned { coordinates ->
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
                val gestureTracker =
                  PopoverAnchorGestureTracker(origin = origin, armDistancePx = armDistancePx)
                val scrollLockHandle = scrollGestureLockState.acquire()

                try {
                  isExpanded = true
                  val startUpdate = gestureTracker.start()
                  scope.pointerState = startUpdate.pointerState
                  if (startUpdate.consumeChange) {
                    // Anchor-originated scrub should own the gesture so parent scroll containers
                    // stay still.
                    press.consume()
                  }

                  val armStartMark = TimeSource.Monotonic.markNow()

                  while (true) {
                    val moveEvent = awaitPointerEvent()
                    val change = moveEvent.changes.find { it.id == press.id } ?: break

                    val currentPos = change.position + anchorWindowOffset
                    val elapsed = armStartMark.elapsedNow().inWholeMilliseconds
                    val update =
                      gestureTracker.update(
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
        }
  ) {
    val progress =
      if (isOverlayVisible) {
        PopoverDefaults.PopoverEasing.transform(animationProgress.value).coerceIn(0f, 1f)
      } else {
        0f
      }

    Box(modifier = Modifier.alpha(1f - progress)) { anchor() }

    if (isOverlayVisible) {
      val placementProvider =
        remember(placement, resolvedScreenPadding) {
          PopoverPlacementProvider(placement, resolvedScreenPadding)
        }
      val holdOverlayAfterReverse = reverseAnimationCompleted && outsideDismissGestureActive
      val heldPaneSize = paneSizeInWindow ?: anchorBounds.size

      Popup(
        popupPositionProvider = placementProvider,
        onDismissRequest = { isExpanded = false },
        properties = PopupProperties(dismissOnClickOutside = false),
      ) {
        if (holdOverlayAfterReverse) {
          Box(
            modifier =
              Modifier.size(
                  width = max(1, heldPaneSize.width).toDp(density),
                  height = max(1, heldPaneSize.height).toDp(density),
                )
                .alpha(0f)
          )
        } else {
          Box(
            modifier =
              Modifier.onGloballyPositioned { coordinates ->
                val positionInWindow = coordinates.positionInWindow()
                paneSizeInWindow = coordinates.size
                paneBoundsInWindow =
                  Rect(
                    left = positionInWindow.x,
                    top = positionInWindow.y,
                    right = positionInWindow.x + coordinates.size.width,
                    bottom = positionInWindow.y + coordinates.size.height,
                  )
              }
          ) {
            PopoverPanePopup(
              anchor = anchor,
              pane = { context(scope) { pane() } },
              anchorBounds = anchorBounds,
              placement = placement,
              progress = progress,
              interactive = scope.acceptsInput,
              collapsedCornerRadius = collapsedCornerRadius,
              screenPadding = resolvedScreenPadding,
              maxWidth = maxWidth,
              minWidth = minWidth,
              expandToMaxWidth = expandToMaxWidth,
            )
          }
        }
      }
    }
  }
}

@Composable
private fun PopoverPanePopup(
  anchor: @Composable () -> Unit,
  pane: @Composable () -> Unit,
  anchorBounds: IntRect,
  placement: PopoverPlacement,
  progress: Float,
  interactive: Boolean,
  collapsedCornerRadius: Dp,
  screenPadding: PopoverScreenPadding,
  maxWidth: Dp?,
  minWidth: Dp,
  expandToMaxWidth: Boolean,
) {
  val density = LocalDensity.current

  SubcomposeLayout(
    modifier = Modifier.then(if (maxWidth != null) Modifier.widthIn(max = maxWidth) else Modifier)
  ) { constraints ->
    val minWidthPx = minWidth.toPx(density).roundToInt()
    val preferredPaneMaxWidth =
      availableWidthForPlacement(
        windowWidth = constraints.maxWidth,
        anchorBounds = anchorBounds,
        screenPadding = screenPadding,
        placement = placement,
      )
    val paneConstraints =
      constraints.copy(
        minWidth = 0,
        minHeight = 0,
        maxWidth =
          min(
            shrinkBounded(constraints.maxWidth, screenPadding.left + screenPadding.right),
            preferredPaneMaxWidth,
          ),
        maxHeight = shrinkBounded(constraints.maxHeight, screenPadding.top + screenPadding.bottom),
      )

    val initialPanePlaceables =
      subcompose(PopoverSlot.InitialMeasurePane) {
          ShrinkWrappedPane(expandToMaxWidth = expandToMaxWidth, content = pane)
        }
        .map { it.measure(paneConstraints) }

    val initiallyMeasuredWidth =
      initialPanePlaceables.maxOfOrNull { it.width } ?: anchorBounds.width
    val initiallyMeasuredHeight =
      initialPanePlaceables.maxOfOrNull { it.height } ?: anchorBounds.height
    val showBelow =
      shouldShowBelow(
        placement = placement,
        childHeight = initiallyMeasuredHeight,
        windowHeight = constraints.maxHeight,
        anchorRect = anchorBounds,
        screenPadding = screenPadding,
      )
    val finalPaneConstraints =
      paneConstraints.copy(
        maxHeight =
          availableHeightForPlacement(
            windowHeight = constraints.maxHeight,
            anchorBounds = anchorBounds,
            screenPadding = screenPadding,
            showBelow = showBelow,
          )
      )
    val finalPanePlaceables =
      subcompose(PopoverSlot.FinalMeasurePane) {
          ShrinkWrappedPane(expandToMaxWidth = expandToMaxWidth, content = pane)
        }
        .map { it.measure(finalPaneConstraints) }

    val paneWidth = finalPanePlaceables.maxOfOrNull { it.width } ?: initiallyMeasuredWidth
    val paneHeight = finalPanePlaceables.maxOfOrNull { it.height } ?: initiallyMeasuredHeight
    val resolvedPaneWidth =
      if (expandToMaxWidth) {
        finalPaneConstraints.maxWidth
      } else {
        paneWidth.coerceAtLeast(minWidthPx).coerceAtMost(finalPaneConstraints.maxWidth)
      }
    val paneSize = IntSize(resolvedPaneWidth, paneHeight)
    val geometry =
      resolvePopoverGeometry(
        anchorBounds = anchorBounds,
        windowSize = IntSize(constraints.maxWidth, constraints.maxHeight),
        placement = placement,
        popupContentSize = paneSize,
        screenPadding = screenPadding,
      )
    val transition =
      PopoverPaneTransition(
        progress = progress,
        anchorContentRect =
          Rect(
            left = geometry.anchorBoundsInPopup.left.toFloat(),
            top = geometry.anchorBoundsInPopup.top.toFloat(),
            right = geometry.anchorBoundsInPopup.right.toFloat(),
            bottom = geometry.anchorBoundsInPopup.bottom.toFloat(),
          ),
      )

    val surfacePlaceable =
      subcompose(PopoverSlot.Surface) {
          CompositionLocalProvider(LocalPopoverPaneTransition provides transition) {
            PopoverPaneSurface(
              anchor = anchor,
              pane = { ShrinkWrappedPane(expandToMaxWidth = expandToMaxWidth, content = pane) },
              paneSize = paneSize,
              anchorContentRect = geometry.anchorBoundsInPopup,
              progress = progress,
              interactive = interactive,
              collapsedCornerRadius = collapsedCornerRadius,
            )
          }
        }
        .single()
        .measure(Constraints.fixed(resolvedPaneWidth, paneHeight))

    layout(resolvedPaneWidth, paneHeight) { surfacePlaceable.place(0, 0) }
  }
}

@Composable
private fun ShrinkWrappedPane(expandToMaxWidth: Boolean = false, content: @Composable () -> Unit) {
  val scrollState = rememberScrollState()
  val edgeAutoScrollState = rememberEdgeAutoScrollState(verticalScrollableState = scrollState)

  CompositionLocalProvider(LocalPopoverPaneEdgeAutoScrollState provides edgeAutoScrollState) {
    Box(
      modifier =
        Modifier.then(
            if (expandToMaxWidth) {
              Modifier.fillMaxWidth()
            } else {
              Modifier.width(IntrinsicSize.Max)
            }
          )
          .edgeAutoScroll(edgeAutoScrollState)
          .verticalScroll(scrollState)
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
  anchorContentRect: IntRect,
  progress: Float,
  interactive: Boolean,
  collapsedCornerRadius: Dp,
) {
  val density = LocalDensity.current
  val anchorSize = anchorContentRect.size
  val animatedWidth =
    sizeForProgress(anchorSize.width.toFloat(), paneSize.width.toFloat(), progress)
  val animatedHeight =
    sizeForProgress(anchorSize.height.toFloat(), paneSize.height.toFloat(), progress)
  val animatedSurfaceSize =
    IntSize(
      width = max(1, animatedWidth.roundToInt()),
      height = max(1, animatedHeight.roundToInt()),
    )
  val surfaceOffset = surfaceOffsetForProgress(anchorContentRect, progress)
  val paneOffset = IntOffset(x = -surfaceOffset.x, y = -surfaceOffset.y)
  val anchorOffset = IntOffset(x = anchorContentRect.left, y = anchorContentRect.top)
  val cornerRadius =
    lerp(
      collapsedCornerRadius.toPx(density),
      PopoverDefaults.ExpandedRadius.toPx(density),
      progress,
    )
  val shape = SquircleShape(cornerRadius.toDp(density))
  val shadowElevation = (12f * progress).dp

  Box(
    modifier =
      Modifier.size(width = paneSize.width.toDp(density), height = paneSize.height.toDp(density))
  ) {
    Box(
      modifier =
        Modifier.offset { surfaceOffset }
          .size(width = animatedWidth.toDp(density), height = animatedHeight.toDp(density))
          .shadow(shadowElevation, shape)
          .clip(shape)
          .background(AppTheme.colors.surfaceRaised, shape)
          .then(
            if (interactive) {
              Modifier
            } else {
              Modifier.pointerInput(Unit) {
                awaitPointerEventScope {
                  while (true) {
                    val event = awaitPointerEvent(pass = PointerEventPass.Initial)
                    event.changes.forEach { it.consume() }
                  }
                }
              }
            }
          )
    ) {
      PopoverCropLayout(
        pane = { Box(modifier = Modifier.alpha(progress)) { pane() } },
        anchor = { Box(modifier = Modifier.alpha(1f - progress)) { anchor() } },
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
      anchorPlaceable.place(x = paneOffset.x + anchorOffset.x, y = paneOffset.y + anchorOffset.y)
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

private fun surfaceOffsetForProgress(anchorContentRect: IntRect, progress: Float): IntOffset {
  return IntOffset(
    x = lerp(anchorContentRect.left.toFloat(), 0f, progress).roundToInt(),
    y = lerp(anchorContentRect.top.toFloat(), 0f, progress).roundToInt(),
  )
}

private fun availableWidthForPlacement(
  windowWidth: Int,
  anchorBounds: IntRect,
  screenPadding: PopoverScreenPadding,
  placement: PopoverPlacement,
): Int {
  if (windowWidth == Constraints.Infinity) {
    return windowWidth
  }

  return when (placement.align) {
    PopoverAlign.Start -> max(0, windowWidth - screenPadding.right - anchorBounds.left)

    PopoverAlign.Center -> max(0, windowWidth - screenPadding.left - screenPadding.right)

    PopoverAlign.End -> max(0, anchorBounds.right - screenPadding.left)
  }
}
