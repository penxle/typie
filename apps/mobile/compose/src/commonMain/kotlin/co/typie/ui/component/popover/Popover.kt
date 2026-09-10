package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.interaction.MutableInteractionSource
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.semantics.hideFromAccessibility
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntRect
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.round
import co.typie.ext.LocalInteractionSource
import co.typie.ext.safeDrawing
import co.typie.ext.toPx
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.component.tooltip.LocalTooltipState
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.launch

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
  screenPadding: PaddingValues = PaddingValues(all = PopoverDefaults.ScreenPadding),
) {
  val density = LocalDensity.current
  val focusManager = LocalFocusManager.current
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
  val anchorSurfaceState = remember { PopoverAnchorSurfaceState() }
  val anchorInteractionSource = remember { MutableInteractionSource() }
  var isExpanded by remember { mutableStateOf(false) }
  var isOverlayVisible by remember { mutableStateOf(false) }
  var anchorBounds by remember { mutableStateOf(IntRect.Zero) }
  var reverseAnimationCompleted by remember { mutableStateOf(false) }
  var paneHasFocus by remember { mutableStateOf(false) }
  val progress = remember { Animatable(0f) }
  val animationScope = rememberCoroutineScope()
  val scope =
    remember(overlayState, overlayOwner, focusManager) {
      PopoverScope(
        onClose = {
          if (overlayState.isOwnedBy(overlayOwner)) {
            if (paneHasFocus) {
              focusManager.clearFocus()
            }
            overlayState.stopAcceptingInput(overlayOwner)
          }
          // Freeze an interrupted intro before the close effect runs on the next frame.
          animationScope.launch(start = CoroutineStart.UNDISPATCHED) { progress.stop() }
          isExpanded = false
        }
      )
    }
  val dismissPopoverFromOutsideGesture by rememberUpdatedState { scope.close() }
  val ownsOverlay = overlayState.isOwnedBy(overlayOwner)
  val latestIsOverlayVisible by rememberUpdatedState(isOverlayVisible)
  val overlayEntry =
    PopoverOverlayEntry(
      owner = overlayOwner,
      progress = progress.asState(),
      placement = placement,
      screenPadding = resolvedScreenPadding,
      anchorSurface = anchorSurfaceState.surface,
      maxWidth = maxWidth,
      minWidth = minWidth,
      pane = {
        val focusObserver =
          if (LocalPopoverPaneRenderPhase.current == PopoverPaneRenderPhase.Interactive) {
            Modifier.onFocusChanged { paneHasFocus = it.hasFocus }
          } else {
            Modifier
          }
        Box(focusObserver) { PopoverPaneSelectionHost(scope = scope, pane = pane) }
      },
      anchor = {
        CompositionLocalProvider(
          LocalTooltipState provides null,
          LocalInteractionSource provides null,
          LocalPopoverAnchorSurfaceState provides null,
          LocalPopoverAnchorContentOnly provides true,
        ) {
          anchor()
        }
      },
    )
  val openPopover = rememberUpdatedState {
    if (!isOverlayVisible) {
      reverseAnimationCompleted = false
      scope.acceptsInput = true
      isOverlayVisible = true
      overlayState.show(owner = overlayOwner, entry = overlayEntry, anchorBounds = anchorBounds)
      focusManager.clearFocus()
      isExpanded = true
    }
  }

  LaunchedEffect(isExpanded) {
    if (isExpanded) {
      progress.animateTo(
        1f,
        tween(PopoverDefaults.ForwardDuration, easing = PopoverDefaults.OpenEasing),
      )
    } else if (isOverlayVisible) {
      reverseAnimationCompleted = false
      progress.animateTo(
        0f,
        tween(PopoverDefaults.ReverseDuration, easing = PopoverDefaults.CloseEasing),
      )
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

  DisposableEffect(overlayState, overlayOwner, focusManager) {
    onDispose {
      if (latestIsOverlayVisible && overlayState.isOwnedBy(overlayOwner)) {
        if (scope.acceptsInput) {
          if (paneHasFocus) {
            focusManager.clearFocus()
          }
          overlayState.stopAcceptingInput(overlayOwner)
        }
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
        interactive = scope.acceptsInput,
      )
    }
  }

  PlatformBackHandler(enabled = isOverlayVisible && ownsOverlay && scope.acceptsInput) {
    scope.close()
  }

  // Keep the original input node alive for press-drag selection. Only the overlay draws
  // the trigger while its surface expands; fading both copies would duplicate its outline.
  val anchorDrawModifier =
    Modifier.semantics { if (isOverlayVisible && ownsOverlay) hideFromAccessibility() }
      .drawWithContent {
        if (!isOverlayVisible || !ownsOverlay || progress.value <= 0f) drawContent()
      }
  val anchorModifier = Modifier.onGloballyPositioned { coordinates ->
    val pos = coordinates.positionInWindow().round()
    anchorBounds = IntRect(pos, coordinates.size)
  }

  if (!enabled) {
    CompositionLocalProvider(LocalTooltipState provides null) {
      Box(modifier = anchorModifier) { Box(modifier = anchorDrawModifier) { anchor() } }
    }
    return
  }

  Box(
    modifier =
      anchorModifier
        .hoverable(anchorInteractionSource)
        .then(
          rememberPopoverTriggerInputModifier(
            interactionSource = anchorInteractionSource,
            canOpen = { !isOverlayVisible && !overlayState.isOutsideDismissGestureActive },
            positionInWindow = {
              it + Offset(anchorBounds.left.toFloat(), anchorBounds.top.toFloat())
            },
            onOpen = { openPopover.value() },
            onSession = { scope.pressGestureSession = it },
          )
        )
  ) {
    CompositionLocalProvider(
      LocalInteractionSource provides anchorInteractionSource,
      LocalPopoverAnchorSurfaceState provides anchorSurfaceState,
      LocalTooltipState provides LocalTooltipState.current.takeUnless { isOverlayVisible },
    ) {
      Box(modifier = anchorDrawModifier) { anchor() }
    }
  }
}
