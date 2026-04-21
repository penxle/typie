package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.round
import co.typie.ext.LocalInteractionSource
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockHandle
import co.typie.ext.safeDrawing
import co.typie.ext.toPx
import co.typie.navigation.PlatformBackHandler
import kotlinx.coroutines.withTimeoutOrNull

@Composable
fun Popover(
  anchor: @Composable () -> Unit,
  pane:
    @Composable
    context(PopoverScope)
    () -> Unit,
  enabled: Boolean = true,
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
  val anchorInteractionSource = remember { MutableInteractionSource() }
  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  var reverseAnimationCompleted by remember { mutableStateOf(false) }
  val progress = remember { Animatable(0f) }
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val scope = remember { PopoverScope(onClose = { isExpanded = false }) }
  val dismissPopoverFromOutsideGesture by rememberUpdatedState { scope.close() }
  val ownsOverlay = overlayState.isOwnedBy(overlayOwner)
  val latestIsOverlayVisible by rememberUpdatedState(isOverlayVisible)
  val latestOwnsOverlay by rememberUpdatedState(ownsOverlay)
  val overlayEntry =
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
    )

  LaunchedEffect(isExpanded) {
    if (isExpanded) {
      reverseAnimationCompleted = false
      scope.acceptsInput = true
      isOverlayVisible = true
      overlayState.show(owner = overlayOwner, entry = overlayEntry, anchorBounds = anchorBounds)
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
    overlayState.isOutsideDismissGestureActive,
  ) {
    if (
      !isExpanded &&
        isOverlayVisible &&
        reverseAnimationCompleted &&
        !overlayState.isOutsideDismissGestureActive
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

  SideEffect {
    if (isExpanded && ownsOverlay) {
      overlayState.updateOutsideDismiss(
        owner = overlayOwner,
        onOutsideDismiss = dismissPopoverFromOutsideGesture,
      )
    } else {
      overlayState.clearOutsideDismiss(overlayOwner)
    }
  }

  SideEffect {
    if (isOverlayVisible && ownsOverlay) {
      overlayState.update(
        owner = overlayOwner,
        entry = overlayEntry,
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
  val anchorModifier = Modifier.onGloballyPositioned { coordinates ->
    val pos = coordinates.positionInWindow().round()
    anchorBounds = IntRect(pos, coordinates.size)
  }

  if (!enabled) {
    Box(modifier = anchorModifier) {
      Box(modifier = Modifier.graphicsLayer { alpha = 1f - easedProgress }) { anchor() }
    }
    return
  }

  Box(
    modifier =
      anchorModifier.pointerInput(Unit) {
        awaitEachGesture {
          val press = awaitFirstDown(requireUnconsumed = false)
          if (overlayState.isOutsideDismissGestureActive) {
            return@awaitEachGesture
          }
          if (isOverlayVisible) {
            return@awaitEachGesture
          }

          val pressInteraction = PressInteraction.Press(press.position)
          anchorInteractionSource.tryEmit(pressInteraction)

          val anchorWindowOffset = Offset(anchorBounds.left.toFloat(), anchorBounds.top.toFloat())
          val initialPositionInWindow = press.position + anchorWindowOffset
          val openTrigger =
            awaitPopoverOpenOrCancellation(
              press = press,
              initialPositionInWindow = initialPositionInWindow,
              touchSlop = viewConfiguration.touchSlop,
              armDelayMillis = PopoverDefaults.ArmDelayMs,
              resolvePositionInWindow = { change -> change.position + anchorWindowOffset },
            )

          when (openTrigger) {
            null -> {
              anchorInteractionSource.tryEmit(PressInteraction.Cancel(pressInteraction))
              return@awaitEachGesture
            }
            is PopoverOpenTrigger.Tap -> {
              anchorInteractionSource.tryEmit(PressInteraction.Release(pressInteraction))
              openTrigger.upChange.consume()
              isExpanded = true
              return@awaitEachGesture
            }
            PopoverOpenTrigger.Pressed -> {}
          }

          var scrollLockHandle: ScrollGestureLockHandle? = null
          var released = false

          try {
            scrollLockHandle = scrollGestureLockState.acquire()
            isExpanded = true
            released =
              trackPressGestureSession(
                pointerId = press.id,
                initialPositionInWindow = initialPositionInWindow,
                downUptimeMillis = press.uptimeMillis,
                armDelayMillis = PopoverDefaults.ArmDelayMs,
                resolvePositionInWindow = { nextChange, _ ->
                  nextChange.position + anchorWindowOffset
                },
              ) { session, change ->
                scope.pressGestureSession = session
                change?.consume()
              }
          } finally {
            anchorInteractionSource.tryEmit(
              if (released) {
                PressInteraction.Release(pressInteraction)
              } else {
                PressInteraction.Cancel(pressInteraction)
              }
            )
            if (!released) {
              scope.pressGestureSession = null
            }
            scrollLockHandle?.release()
          }
        }
      }
  ) {
    CompositionLocalProvider(LocalInteractionSource provides anchorInteractionSource) {
      Box(modifier = Modifier.graphicsLayer { alpha = 1f - easedProgress }) { anchor() }
    }
  }
}

private sealed interface PopoverOpenTrigger {
  data class Tap(val upChange: PointerInputChange) : PopoverOpenTrigger

  data object Pressed : PopoverOpenTrigger
}

private suspend fun AwaitPointerEventScope.awaitPopoverOpenOrCancellation(
  press: PointerInputChange,
  initialPositionInWindow: Offset,
  touchSlop: Float,
  armDelayMillis: Long,
  resolvePositionInWindow: (PointerInputChange) -> Offset,
): PopoverOpenTrigger? {
  var elapsedMillis = 0L

  while (elapsedMillis < armDelayMillis) {
    val event = withTimeoutOrNull(armDelayMillis - elapsedMillis) { awaitPointerEvent() }
    if (event == null) {
      return PopoverOpenTrigger.Pressed
    }

    val change = event.changes.find { it.id == press.id } ?: return null
    val currentPositionInWindow = resolvePositionInWindow(change)
    elapsedMillis = change.uptimeMillis - press.uptimeMillis
    val dragDistance = (currentPositionInWindow - initialPositionInWindow).getDistance()

    if (dragDistance > touchSlop) {
      return null
    }
    if (!change.pressed) {
      return PopoverOpenTrigger.Tap(change)
    }
  }

  return PopoverOpenTrigger.Pressed
}
