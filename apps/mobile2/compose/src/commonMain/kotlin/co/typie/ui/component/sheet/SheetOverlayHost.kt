package co.typie.ui.component.sheet

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.Velocity
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.clickable
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.component.ResponsiveContainer
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

@Composable
internal fun SheetOverlayHosts(state: SheetOverlayPresenterState) {
  state.entries.forEach { entry ->
    key(entry) {
      @Suppress("UNCHECKED_CAST") SheetOverlayHost(entry = entry as SheetOverlayEntry<Any?>)
    }
  }
}

@Composable
private fun <R> SheetOverlayHost(entry: SheetOverlayEntry<R>) {
  val viewModelStore = remember { ViewModelStore() }
  val viewModelStoreOwner = remember {
    object : ViewModelStoreOwner {
      override val viewModelStore
        get() = viewModelStore
    }
  }
  DisposableEffect(Unit) { onDispose { viewModelStore.clear() } }

  val density = LocalDensity.current
  val coroutineScope = rememberCoroutineScope()
  val haptics = rememberSheetHaptics()
  val focusRequester = remember { FocusRequester() }
  val progress = remember { Animatable(0f) }
  val exitSpec = remember {
    tween<Float>(durationMillis = SheetDefaults.ExitDuration, easing = SheetDefaults.ExitEasing)
  }

  var measuredSheetHeightPx by remember { mutableFloatStateOf(0f) }
  var dragOffsetPx by remember { mutableFloatStateOf(0f) }
  var lastSettledDetentId by remember { mutableStateOf<SheetDetentId?>(null) }
  var isResolving by remember { mutableStateOf(false) }

  val sheetScope =
    remember(entry) {
      object : SheetScope<R> {
        override val controller: SheetController<R> = entry.controller

        override fun complete(result: R) {
          entry.controller.complete(result)
        }

        override fun dismiss(reason: SheetDismissReason) {
          entry.controller.dismiss(reason)
        }
      }
    }

  LaunchedEffect(entry) {
    if (entry.spec.haptics.onPresent) {
      haptics.perform(SheetHapticEvent.Present)
    }
  }

  LaunchedEffect(entry.controller.resolutionRequest) {
    val request = entry.controller.resolutionRequest ?: return@LaunchedEffect
    if (isResolving) return@LaunchedEffect
    isResolving = true
    progress.animateTo(targetValue = 0f, animationSpec = exitSpec)
    entry.resolve(request)
  }

  BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
    val requiresContentMeasurement =
      remember(entry.spec.sizePolicy) { entry.spec.sizePolicy.requiresContentMeasurement() }
    val resolvedDetents =
      remember(entry.spec.sizePolicy, maxHeight, measuredSheetHeightPx) {
        resolveDetentsForSheetMeasurement(
          policy = entry.spec.sizePolicy,
          viewportHeight = maxHeight,
          measuredSheetHeightPx = measuredSheetHeightPx,
          density = density,
        )
      }
    val initialDetentId = entry.spec.sizePolicy.initialDetentId()
    val initialResolvedDetent =
      resolvedDetents.firstOrNull { it.id == initialDetentId } ?: resolvedDetents.firstOrNull()
    val minDetentHeightPx = resolvedDetents.minOfOrNull { with(density) { it.height.toPx() } } ?: 0f
    val maxDetentHeightPx = resolvedDetents.maxOfOrNull { with(density) { it.height.toPx() } } ?: 0f
    val maxSheetHeight = maxHeight
    val isContentReady = !requiresContentMeasurement || measuredSheetHeightPx > 0f
    val initialSheetHeightPx =
      resolveInitialSheetHeightPx(
        density = density,
        requiresContentMeasurement = requiresContentMeasurement,
        initialResolvedDetent = initialResolvedDetent,
      )
    var sheetHeightPx by
      remember(entry, requiresContentMeasurement, initialResolvedDetent?.id) {
        mutableFloatStateOf(initialSheetHeightPx)
      }
    val renderedSheetHeightPx =
      resolveRenderedSheetHeightPx(
        currentSheetHeightPx = sheetHeightPx,
        requiresContentMeasurement = requiresContentMeasurement,
        measuredSheetHeightPx = measuredSheetHeightPx,
        initialSheetHeightPx = initialSheetHeightPx,
      )
    val sheetSurfaceAlpha =
      resolveSheetSurfaceAlpha(
        requiresContentMeasurement = requiresContentMeasurement,
        isContentReady = isContentReady,
      )
    val isSheetMeasurementConstrained =
      shouldTreatMeasuredSheetAsConstrained(
        requiresContentMeasurement = requiresContentMeasurement,
        measuredSheetHeightPx = measuredSheetHeightPx,
        renderedSheetHeightPx = renderedSheetHeightPx,
      )

    LaunchedEffect(resolvedDetents, entry.isTopOfStack) {
      if (resolvedDetents.isEmpty()) return@LaunchedEffect
      entry.controller.updateResolvedDetents(
        detents = resolvedDetents,
        initialDetentId = initialResolvedDetent?.id ?: initialDetentId,
        stackDepth = entry.controller.stackDepth,
        isTopOfStack = entry.isTopOfStack,
      )
      val initialHeightPx = initialResolvedDetent?.let { with(density) { it.height.toPx() } } ?: 0f
      if (sheetHeightPx == 0f && initialHeightPx > 0f) {
        sheetHeightPx = initialHeightPx
        dragOffsetPx = 0f
        entry.controller.animateTo(initialResolvedDetent?.id ?: initialDetentId)
        entry.controller.snapToCurrentTarget()
      }
    }

    LaunchedEffect(isContentReady, sheetHeightPx > 0f) {
      if (isContentReady && sheetHeightPx > 0f && progress.value == 0f) {
        progress.animateTo(
          1f,
          animationSpec =
            tween(durationMillis = SheetDefaults.EnterDuration, easing = SheetDefaults.EnterEasing),
        )
      }
    }

    LaunchedEffect(entry.controller.targetDetentId, resolvedDetents) {
      if (!isContentReady || resolvedDetents.isEmpty()) return@LaunchedEffect
      val targetDetent =
        resolvedDetents.firstOrNull { it.id == entry.controller.targetDetentId }
          ?: initialResolvedDetent
          ?: return@LaunchedEffect
      val targetHeightPx = with(density) { targetDetent.height.toPx() }
      if (abs(sheetHeightPx - targetHeightPx) <= 0.5f && dragOffsetPx <= 0.5f) {
        sheetHeightPx = targetHeightPx
        dragOffsetPx = 0f
        entry.controller.snapToCurrentTarget()
        lastSettledDetentId = targetDetent.id
        return@LaunchedEffect
      }
      val heightAnimatable = Animatable(sheetHeightPx)
      val dragOffsetAnimatable = Animatable(dragOffsetPx)
      launch {
        heightAnimatable.animateTo(
          targetHeightPx,
          animationSpec =
            tween(
              durationMillis = SheetDefaults.HeightAnimationDuration,
              easing = SheetDefaults.EnterEasing,
            ),
        ) {
          sheetHeightPx = value
        }
      }
      launch {
        dragOffsetAnimatable.animateTo(
          0f,
          animationSpec =
            tween(
              durationMillis = SheetDefaults.HeightAnimationDuration,
              easing = SheetDefaults.EnterEasing,
            ),
        ) {
          dragOffsetPx = value
        }
      }
      if (entry.spec.haptics.onDetentSnap && lastSettledDetentId != targetDetent.id) {
        haptics.perform(SheetHapticEvent.DetentSnap)
      }
      entry.controller.snapToCurrentTarget()
      lastSettledDetentId = targetDetent.id
    }

    val visibleFraction =
      resolveSheetVisibleFraction(
        progress = progress.value,
        renderedSheetHeightPx = renderedSheetHeightPx,
        dragOffsetPx = dragOffsetPx,
      )

    SideEffect { entry.controller.updateVisibleFraction(visibleFraction) }

    fun settleOrDismiss(velocity: Float) {
      if (isResolving) return
      val effectiveSheetHeightPx = (sheetHeightPx - dragOffsetPx).coerceAtLeast(0f)
      val effectiveSheetHeight = with(density) { effectiveSheetHeightPx.toDp() }

      if (
        entry.spec.dismissPolicy.dragDown &&
          shouldDismissDraggedSheet(
            policy = entry.spec.sizePolicy,
            detents = resolvedDetents,
            currentDetentId = entry.controller.currentDetentId,
            sheetHeight = effectiveSheetHeight,
            velocity = velocity,
          )
      ) {
        if (entry.spec.haptics.onDismiss) {
          haptics.perform(SheetHapticEvent.Dismiss)
        }
        entry.controller.dismiss(SheetDismissReason.Drag)
        return
      }

      val nearest =
        resolveSheetSettledDetent(
          policy = entry.spec.sizePolicy,
          detents = resolvedDetents,
          currentDetentId = entry.controller.currentDetentId,
          sheetHeight = effectiveSheetHeight,
          velocity = velocity,
        ) ?: return
      if (nearest.id == entry.controller.targetDetentId) {
        coroutineScope.launch {
          val heightAnimatable = Animatable(sheetHeightPx)
          val dragOffsetAnimatable = Animatable(dragOffsetPx)
          launch {
            heightAnimatable.animateTo(
              with(density) { nearest.height.toPx() },
              animationSpec =
                tween(
                  durationMillis = SheetDefaults.HeightAnimationDuration,
                  easing = SheetDefaults.EnterEasing,
                ),
            ) {
              sheetHeightPx = value
            }
          }
          launch {
            dragOffsetAnimatable.animateTo(
              0f,
              animationSpec =
                tween(
                  durationMillis = SheetDefaults.HeightAnimationDuration,
                  easing = SheetDefaults.EnterEasing,
                ),
            ) {
              dragOffsetPx = value
            }
          }
          if (entry.spec.haptics.onDetentSnap && lastSettledDetentId != nearest.id) {
            haptics.perform(SheetHapticEvent.DetentSnap)
          }
          entry.controller.snapToCurrentTarget()
          lastSettledDetentId = nearest.id
        }
      } else {
        if (dragOffsetPx > 0f) {
          coroutineScope.launch {
            val dragOffsetAnimatable = Animatable(dragOffsetPx)
            dragOffsetAnimatable.animateTo(
              0f,
              animationSpec =
                tween(
                  durationMillis = SheetDefaults.HeightAnimationDuration,
                  easing = SheetDefaults.EnterEasing,
                ),
            ) {
              dragOffsetPx = value
            }
          }
        }
        entry.controller.animateTo(nearest.id)
      }
    }

    val nestedScrollConnection =
      remember(entry, entry.isTopOfStack, minDetentHeightPx, maxDetentHeightPx) {
        object : NestedScrollConnection {
          override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
            if (
              !entry.isTopOfStack || isResolving || available.y >= 0f || maxDetentHeightPx <= 0f
            ) {
              return Offset.Zero
            }
            val nextState =
              consumeSheetDragDelta(
                currentHeightPx = sheetHeightPx,
                currentOffsetPx = dragOffsetPx,
                delta = available.y,
                minHeightPx = minDetentHeightPx,
                maxHeightPx = maxDetentHeightPx,
              )
            val consumed =
              resolveConsumedSheetScrollDeltaY(
                currentHeightPx = sheetHeightPx,
                currentOffsetPx = dragOffsetPx,
                nextState = nextState,
              )
            if (abs(consumed) <= 0.5f) return Offset.Zero
            sheetHeightPx = nextState.heightPx
            dragOffsetPx = nextState.offsetPx
            return Offset(0f, consumed)
          }

          override fun onPostScroll(
            consumed: Offset,
            available: Offset,
            source: NestedScrollSource,
          ): Offset {
            if (!entry.isTopOfStack || isResolving || available.y <= 0f) {
              return Offset.Zero
            }
            val nextState =
              consumeSheetDragDelta(
                currentHeightPx = sheetHeightPx,
                currentOffsetPx = dragOffsetPx,
                delta = available.y,
                minHeightPx = minDetentHeightPx,
                maxHeightPx = maxDetentHeightPx,
              )
            val consumedY =
              resolveConsumedSheetScrollDeltaY(
                currentHeightPx = sheetHeightPx,
                currentOffsetPx = dragOffsetPx,
                nextState = nextState,
              )
            if (abs(consumedY) <= 0.5f) return Offset.Zero
            sheetHeightPx = nextState.heightPx
            dragOffsetPx = nextState.offsetPx
            return Offset(0f, consumedY)
          }

          override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
            if (!entry.isTopOfStack || isResolving || resolvedDetents.isEmpty()) {
              return Velocity.Zero
            }
            settleOrDismiss(velocity = available.y.takeIf { it != 0f } ?: consumed.y)
            return Velocity.Zero
          }
        }
      }

    val draggableState = rememberDraggableState { delta ->
      if (!entry.isTopOfStack || isResolving) return@rememberDraggableState
      val nextState =
        consumeSheetDragDelta(
          currentHeightPx = sheetHeightPx,
          currentOffsetPx = dragOffsetPx,
          delta = delta,
          minHeightPx = minDetentHeightPx,
          maxHeightPx = maxDetentHeightPx.coerceAtLeast(sheetHeightPx),
        )
      sheetHeightPx = nextState.heightPx
      dragOffsetPx = nextState.offsetPx
    }
    val dragRegionModifier =
      Modifier.draggable(
        state = draggableState,
        orientation = Orientation.Vertical,
        enabled = entry.isTopOfStack && !isResolving,
        onDragStopped = { velocity -> settleOrDismiss(velocity) },
      )

    PlatformBackHandler(
      enabled =
        !isResolving &&
          entry.isTopOfStack &&
          entry.spec.dismissPolicy.back &&
          entry.mode == SheetMode.Modal
    ) {
      entry.controller.dismiss(SheetDismissReason.Back)
    }

    val showScrim =
      entry.mode == SheetMode.Modal &&
        entry.isTopOfStack &&
        entry.spec.chrome.scrim.visible &&
        visibleFraction > 0f
    val scrimAlpha = visibleFraction * entry.spec.chrome.scrim.opacity
    val currentColors = AppTheme.colors
    val totalOffsetY =
      resolveSheetOffsetY(
        progress = progress.value,
        renderedSheetHeightPx = renderedSheetHeightPx,
        dragOffsetPx = dragOffsetPx,
      )

    Box(
      modifier =
        Modifier.fillMaxSize()
          .onPreviewKeyEvent { event ->
            if (
              entry.isTopOfStack &&
                entry.mode == SheetMode.Modal &&
                entry.spec.dismissPolicy.back &&
                !isResolving &&
                event.type == KeyEventType.KeyDown &&
                event.key == Key.Escape
            ) {
              entry.controller.dismiss(SheetDismissReason.Back)
              true
            } else {
              false
            }
          }
          .focusRequester(focusRequester)
          .focusable(enabled = !isResolving && entry.isTopOfStack && entry.mode == SheetMode.Modal),
      contentAlignment = Alignment.BottomCenter,
    ) {
      if (showScrim) {
        val scrimModifier = Modifier.fillMaxSize().alpha(scrimAlpha).background(currentColors.scrim)

        if (
          !isResolving &&
            entry.spec.chrome.scrim.blocksPointerInput &&
            entry.spec.dismissPolicy.outsideTap
        ) {
          Box(
            modifier =
              scrimModifier.clickable { entry.controller.dismiss(SheetDismissReason.OutsideTap) }
          )
        } else {
          Box(modifier = scrimModifier)
        }
      }

      ResponsiveContainer(modifier = Modifier.fillMaxWidth(), alignment = Alignment.BottomCenter) {
        Column(
          horizontalAlignment = Alignment.CenterHorizontally,
          modifier =
            Modifier.fillMaxWidth()
              .heightIn(max = maxSheetHeight)
              .then(
                if (renderedSheetHeightPx > 0f) {
                  Modifier.height(with(density) { renderedSheetHeightPx.toDp() })
                } else {
                  Modifier
                }
              )
              .onSizeChanged { size ->
                if (requiresContentMeasurement) {
                  val nextHeightPx = size.height.toFloat()
                  if (!isSheetMeasurementConstrained || nextHeightPx > measuredSheetHeightPx) {
                    measuredSheetHeightPx = nextHeightPx
                  }
                }
              }
              .offset { IntOffset(0, totalOffsetY) }
              .alpha(sheetSurfaceAlpha)
              .dropShadow(
                RoundedCornerShape(
                  topStart = entry.spec.chrome.topCornerRadius,
                  topEnd = entry.spec.chrome.topCornerRadius,
                )
              ) {
                color = currentColors.shadowAmbient
                radius = 8f
              }
              .dropShadow(
                RoundedCornerShape(
                  topStart = entry.spec.chrome.topCornerRadius,
                  topEnd = entry.spec.chrome.topCornerRadius,
                )
              ) {
                color = currentColors.shadow
                offset = Offset(0f, -4f)
                radius = 12f
              }
              .clip(
                RoundedCornerShape(
                  topStart = entry.spec.chrome.topCornerRadius,
                  topEnd = entry.spec.chrome.topCornerRadius,
                )
              )
              .background(currentColors.surfaceRaised),
        ) {
          when (val handle = entry.spec.chrome.handle) {
            SheetHandleStyle.Hidden -> Unit
            is SheetHandleStyle.Visible -> {
              Box(
                modifier =
                  Modifier.then(dragRegionModifier)
                    .fillMaxWidth()
                    .height(handle.topPadding + handle.height + handle.bottomPadding),
                contentAlignment = Alignment.Center,
              ) {
                Box(
                  modifier =
                    Modifier.size(width = handle.width, height = handle.height)
                      .clip(RoundedCornerShape(handle.height / 2))
                      .background(currentColors.borderSubtle)
                )
              }
            }
          }

          Box(
            modifier =
              Modifier.fillMaxWidth().weight(1f, fill = false).nestedScroll(nestedScrollConnection)
          ) {
            CompositionLocalProvider(
              LocalViewModelStoreOwner provides viewModelStoreOwner,
              LocalSheetDragRegionModifier provides dragRegionModifier,
            ) {
              Column(modifier = Modifier.fillMaxWidth()) { entry.content.invoke(sheetScope) }
            }
          }
        }
      }
    }

    LaunchedEffect(entry.isTopOfStack, entry.mode, progress.value) {
      if (entry.isTopOfStack && entry.mode == SheetMode.Modal && progress.value > 0f) {
        focusRequester.requestFocus()
      }
    }
  }
}

internal fun resolveInitialSheetHeightPx(
  density: Density,
  requiresContentMeasurement: Boolean,
  initialResolvedDetent: ResolvedSheetDetent?,
): Float =
  when {
    requiresContentMeasurement -> 0f
    initialResolvedDetent != null -> with(density) { initialResolvedDetent.height.toPx() }
    else -> 0f
  }

internal fun resolveRenderedSheetHeightPx(
  currentSheetHeightPx: Float,
  requiresContentMeasurement: Boolean,
  measuredSheetHeightPx: Float,
  initialSheetHeightPx: Float,
): Float =
  when {
    currentSheetHeightPx > 0f -> currentSheetHeightPx
    requiresContentMeasurement -> measuredSheetHeightPx
    else -> initialSheetHeightPx
  }

internal fun resolveSheetSurfaceAlpha(
  requiresContentMeasurement: Boolean,
  isContentReady: Boolean,
): Float =
  if (requiresContentMeasurement && !isContentReady) {
    0f
  } else {
    1f
  }

internal fun shouldTreatMeasuredSheetAsConstrained(
  requiresContentMeasurement: Boolean,
  measuredSheetHeightPx: Float,
  renderedSheetHeightPx: Float,
): Boolean {
  if (!requiresContentMeasurement || measuredSheetHeightPx <= 0f) {
    return false
  }
  return renderedSheetHeightPx + 0.5f < measuredSheetHeightPx
}

internal data class SheetDragState(val heightPx: Float, val offsetPx: Float)

internal fun resolveConsumedSheetScrollDeltaY(
  currentHeightPx: Float,
  currentOffsetPx: Float,
  nextState: SheetDragState,
): Float = (currentHeightPx - nextState.heightPx) + (nextState.offsetPx - currentOffsetPx)

internal fun consumeSheetDragDelta(
  currentHeightPx: Float,
  currentOffsetPx: Float,
  delta: Float,
  minHeightPx: Float,
  maxHeightPx: Float,
): SheetDragState {
  if (delta == 0f) {
    return SheetDragState(heightPx = currentHeightPx, offsetPx = currentOffsetPx)
  }

  var nextHeightPx = currentHeightPx
  var nextOffsetPx = currentOffsetPx

  if (delta > 0f) {
    val collapsibleHeight = (nextHeightPx - minHeightPx).coerceAtLeast(0f)
    val heightDelta = minOf(delta, collapsibleHeight)
    nextHeightPx -= heightDelta

    val remainingDelta = delta - heightDelta
    if (remainingDelta > 0f) {
      nextOffsetPx += remainingDelta
    }
  } else {
    var remainingDelta = -delta

    val offsetRecovery = minOf(remainingDelta, nextOffsetPx)
    nextOffsetPx -= offsetRecovery
    remainingDelta -= offsetRecovery

    if (remainingDelta > 0f) {
      val expandableHeight = (maxHeightPx - nextHeightPx).coerceAtLeast(0f)
      val heightDelta = minOf(remainingDelta, expandableHeight)
      nextHeightPx += heightDelta
    }
  }

  return SheetDragState(
    heightPx = nextHeightPx.coerceAtLeast(0f),
    offsetPx = nextOffsetPx.coerceAtLeast(0f),
  )
}

internal fun resolveSheetOffsetY(
  progress: Float,
  renderedSheetHeightPx: Float,
  dragOffsetPx: Float,
): Int = (((1f - progress) * renderedSheetHeightPx) + dragOffsetPx).roundToInt()

internal fun resolveSheetVisibleFraction(
  progress: Float,
  renderedSheetHeightPx: Float,
  dragOffsetPx: Float,
): Float {
  if (renderedSheetHeightPx <= 0f) {
    return 0f
  }

  val totalOffsetPx = ((1f - progress.coerceIn(0f, 1f)) * renderedSheetHeightPx) + dragOffsetPx
  return (1f - (totalOffsetPx / renderedSheetHeightPx)).coerceIn(0f, 1f)
}

internal fun resolveDetentsForSheetMeasurement(
  policy: SheetSizePolicy,
  viewportHeight: Dp,
  measuredSheetHeightPx: Float,
  density: Density,
): List<ResolvedSheetDetent> {
  val sheetHeight =
    if (measuredSheetHeightPx > 0f) {
      with(density) { measuredSheetHeightPx.toDp() }
    } else {
      null
    }

  return when (policy) {
    is SheetSizePolicy.Intrinsic -> {
      val measured = sheetHeight ?: return emptyList()
      SheetDetentResolver.resolve(
        policy = policy,
        context = SheetDetentContext(viewportHeight = viewportHeight, contentHeight = measured),
      )
    }

    is SheetSizePolicy.Fixed,
    is SheetSizePolicy.Max ->
      SheetDetentResolver.resolve(
        policy = policy,
        context =
          SheetDetentContext(
            viewportHeight = viewportHeight,
            contentHeight = sheetHeight ?: viewportHeight,
          ),
      )

    is SheetSizePolicy.Detents -> {
      val availableDetents =
        (listOf(policy.initial) + policy.available).filter { detent ->
          sheetHeight != null || !detent.requiresContentMeasurement()
        }
      if (availableDetents.isEmpty()) {
        return emptyList()
      }
      availableDetents
        .map { detent ->
          SheetDetentResolver.resolveDetent(
            detent = detent,
            context =
              SheetDetentContext(
                viewportHeight = viewportHeight,
                contentHeight = sheetHeight ?: viewportHeight,
              ),
          )
        }
        .distinctBy { it.id }
        .sortedBy { it.height.value }
    }
  }
}
