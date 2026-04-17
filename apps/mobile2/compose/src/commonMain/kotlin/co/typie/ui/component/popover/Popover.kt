package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.round
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.safeDrawing
import co.typie.ext.toPx
import co.typie.navigation.PlatformBackHandler

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

  val overlayState = LocalPopoverOverlayState.current
  val overlayOwner = remember { Any() }
  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  var outsideDismissGestureActive by remember { mutableStateOf(false) }
  var reverseAnimationCompleted by remember { mutableStateOf(false) }
  val progress = remember { Animatable(0f) }
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val outsideTapHostState = LocalPopoverOutsideTapHostState.current
  val scope = remember { PopoverScope(onClose = { isExpanded = false }) }
  val dismissPopoverFromOutsideGesture by rememberUpdatedState {
    outsideDismissGestureActive = true
    scope.close()
  }
  val finishOutsideDismissGesture by rememberUpdatedState { outsideDismissGestureActive = false }
  var outsideTapHostHandle by remember { mutableStateOf<PopoverOutsideTapHostHandle?>(null) }
  val ownsOverlay = overlayState.isOwnedBy(overlayOwner)
  val latestIsOverlayVisible by rememberUpdatedState(isOverlayVisible)
  val latestOwnsOverlay by rememberUpdatedState(ownsOverlay)

  LaunchedEffect(isExpanded) {
    if (isExpanded) {
      outsideDismissGestureActive = false
      reverseAnimationCompleted = false
      scope.acceptsInput = true
      isOverlayVisible = true
      overlayState.show(
        owner = overlayOwner,
        entry =
          PopoverOverlayEntry(
            owner = overlayOwner,
            placement = placement,
            screenPadding = resolvedScreenPadding,
            collapsedCornerRadius = collapsedCornerRadius,
            maxWidth = maxWidth,
            minWidth = minWidth,
            expandToMaxWidth = expandToMaxWidth,
            pane = { PopoverPaneSelectionHost(scope = scope, pane = pane) },
            anchor = { anchor() },
          ),
        anchorBounds = anchorBounds,
      )
      progress.stop()
      progress.snapTo(0f)
      progress.animateTo(1f, tween(PopoverDefaults.ForwardDuration, easing = LinearEasing))
    } else if (isOverlayVisible) {
      val from = if (progress.value == 0f) 1f else progress.value
      reverseAnimationCompleted = false
      progress.stop()
      progress.snapTo(from)
      progress.animateTo(0f, tween(PopoverDefaults.ReverseDuration, easing = LinearEasing))
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
      overlayState.clear(overlayOwner)
      scope.pressGestureSession = null
      reverseAnimationCompleted = false
    }
  }

  DisposableEffect(overlayState, overlayOwner) {
    onDispose {
      if (latestIsOverlayVisible && latestOwnsOverlay) {
        overlayState.detach(overlayOwner)
      } else {
        overlayState.clear(overlayOwner)
      }
      scope.pressGestureSession = null
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
    if (isExpanded && ownsOverlay) {
      outsideTapHostHandle?.update(
        paneBounds = overlayState.paneBoundsInWindow,
        onDismiss = dismissPopoverFromOutsideGesture,
        onDismissGestureFinished = finishOutsideDismissGesture,
      )
    }
  }

  SideEffect {
    if (isOverlayVisible && ownsOverlay) {
      overlayState.update(
        owner = overlayOwner,
        anchorBounds = anchorBounds,
        progress = progress.value.coerceIn(0f, 1f),
        interactive = scope.acceptsInput,
      )
    }
  }

  PlatformBackHandler(enabled = isOverlayVisible && ownsOverlay) { isExpanded = false }

  val easedProgress =
    if (isOverlayVisible && ownsOverlay) {
      PopoverDefaults.PopoverEasing.transform(progress.value).coerceIn(0f, 1f)
    } else {
      0f
    }

  Box(
    modifier =
      Modifier.onGloballyPositioned { coordinates ->
          val pos = coordinates.positionInWindow().round()
          anchorBounds = IntRect(pos, coordinates.size)
        }
        .pointerInput(Unit) {
          awaitEachGesture {
            val press = awaitFirstDown(requireUnconsumed = false)
            if (isOverlayVisible) {
              return@awaitEachGesture
            }

            val anchorWindowOffset = Offset(anchorBounds.left.toFloat(), anchorBounds.top.toFloat())
            val scrollLockHandle = scrollGestureLockState.acquire()
            var released = false

            try {
              isExpanded = true
              press.consume()

              released =
                trackPressGestureSession(
                  pointerId = press.id,
                  initialPositionInWindow = press.position + anchorWindowOffset,
                  downUptimeMillis = press.uptimeMillis,
                  armDelayMillis = PopoverDefaults.ArmDelayMs,
                  resolvePositionInWindow = { change, _ -> change.position + anchorWindowOffset },
                ) { session, change ->
                  scope.pressGestureSession = session
                  change?.consume()
                }
            } finally {
              if (!released) {
                scope.pressGestureSession = null
              }
              scrollLockHandle.release()
            }
          }
        }
  ) {
    Box(modifier = Modifier.graphicsLayer { alpha = 1f - easedProgress }) { anchor() }
  }
}
