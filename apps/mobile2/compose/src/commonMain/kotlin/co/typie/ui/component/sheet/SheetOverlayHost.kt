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
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.clickable
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.component.ResponsiveContainer
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import kotlin.math.abs
import kotlin.math.roundToInt

@Composable
internal fun SheetOverlayHosts(
  state: SheetOverlayPresenterState,
) {
  state.entries.forEach { entry ->
    key(entry) {
      @Suppress("UNCHECKED_CAST")
      SheetOverlayHost(entry = entry as SheetOverlayEntry<Any?>)
    }
  }
}

@Composable
private fun <R> SheetOverlayHost(
  entry: SheetOverlayEntry<R>,
) {
  val viewModelStore = remember { ViewModelStore() }
  val viewModelStoreOwner = remember {
    object : ViewModelStoreOwner {
      override val viewModelStore get() = viewModelStore
    }
  }
  DisposableEffect(Unit) {
    onDispose { viewModelStore.clear() }
  }

  val density = LocalDensity.current
  val coroutineScope = rememberCoroutineScope()
  val haptics = rememberSheetHaptics()
  val focusRequester = remember { FocusRequester() }
  val progress = remember { Animatable(0f) }
  val exitSpec = remember {
    tween<Float>(
      durationMillis = SheetDefaults.ExitDuration,
      easing = SheetDefaults.ExitEasing,
    )
  }

  var measuredContentHeightPx by remember { mutableFloatStateOf(0f) }
  var lastSettledDetentId by remember { mutableStateOf<SheetDetentId?>(null) }
  var isResolving by remember { mutableStateOf(false) }

  val sheetScope = remember(entry) {
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
    progress.animateTo(
      targetValue = 0f,
      animationSpec = exitSpec,
    )
    entry.resolve(request)
  }

  BoxWithConstraints(
    modifier = Modifier.fillMaxSize(),
  ) {
    val requiresContentMeasurement = remember(entry.spec.sizePolicy) {
      entry.spec.sizePolicy.requiresContentMeasurement()
    }
    val viewportHeight = maxHeight
    val contentHeight = if (requiresContentMeasurement && measuredContentHeightPx > 0f) {
      with(density) { measuredContentHeightPx.toDp() }
    } else {
      viewportHeight
    }
    val resolvedDetents = remember(
      entry.spec.sizePolicy,
      viewportHeight,
      if (requiresContentMeasurement) contentHeight else viewportHeight,
    ) {
      SheetDetentResolver.resolve(
        policy = entry.spec.sizePolicy,
        context = SheetDetentContext(
          viewportHeight = viewportHeight,
          contentHeight = contentHeight,
        ),
      )
    }
    val initialDetentId = entry.spec.sizePolicy.initialDetentId()
    val initialResolvedDetent = resolvedDetents.firstOrNull { it.id == initialDetentId } ?: resolvedDetents.firstOrNull()
    val minDetentHeightPx = resolvedDetents.minOfOrNull { with(density) { it.height.toPx() } } ?: 0f
    val maxDetentHeightPx = resolvedDetents.maxOfOrNull { with(density) { it.height.toPx() } } ?: 0f
    val maxSheetHeight = maxHeight
    val isContentReady = !requiresContentMeasurement || measuredContentHeightPx > 0f
    val initialSheetHeightPx = resolveInitialSheetHeightPx(
      density = density,
      requiresContentMeasurement = requiresContentMeasurement,
      initialResolvedDetent = initialResolvedDetent,
    )
    var sheetHeightPx by remember(entry, requiresContentMeasurement, initialResolvedDetent?.id) {
      mutableFloatStateOf(initialSheetHeightPx)
    }
    val renderedSheetHeightPx = resolveRenderedSheetHeightPx(
      currentSheetHeightPx = sheetHeightPx,
      requiresContentMeasurement = requiresContentMeasurement,
      measuredContentHeightPx = measuredContentHeightPx,
      initialSheetHeightPx = initialSheetHeightPx,
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
        entry.controller.animateTo(initialResolvedDetent?.id ?: initialDetentId)
        entry.controller.snapToCurrentTarget()
      }
    }

    LaunchedEffect(isContentReady, sheetHeightPx > 0f) {
      if (isContentReady && sheetHeightPx > 0f && progress.value == 0f) {
        progress.animateTo(
          1f,
          animationSpec = tween(
            durationMillis = SheetDefaults.EnterDuration,
            easing = SheetDefaults.EnterEasing,
          ),
        )
      }
    }

    LaunchedEffect(entry.controller.targetDetentId, resolvedDetents) {
      if (!isContentReady || resolvedDetents.isEmpty()) return@LaunchedEffect
      val targetDetent = resolvedDetents.firstOrNull { it.id == entry.controller.targetDetentId }
        ?: initialResolvedDetent
        ?: return@LaunchedEffect
      val targetHeightPx = with(density) { targetDetent.height.toPx() }
      if (abs(sheetHeightPx - targetHeightPx) <= 0.5f) {
        sheetHeightPx = targetHeightPx
        entry.controller.snapToCurrentTarget()
        lastSettledDetentId = targetDetent.id
        return@LaunchedEffect
      }
      val animatable = Animatable(sheetHeightPx)
      animatable.animateTo(
        targetHeightPx,
        animationSpec = tween(
          durationMillis = SheetDefaults.HeightAnimationDuration,
          easing = SheetDefaults.EnterEasing,
        ),
      ) {
        sheetHeightPx = value
      }
      if (entry.spec.haptics.onDetentSnap && lastSettledDetentId != targetDetent.id) {
        haptics.perform(SheetHapticEvent.DetentSnap)
      }
      entry.controller.snapToCurrentTarget()
      lastSettledDetentId = targetDetent.id
    }

    SideEffect {
      entry.controller.updateVisibleFraction(progress.value)
    }

    fun settleOrDismiss(velocity: Float) {
      if (isResolving) return

      if (
        entry.spec.dismissPolicy.dragDown &&
        shouldDismissDraggedSheet(
          policy = entry.spec.sizePolicy,
          detents = resolvedDetents,
          currentDetentId = entry.controller.currentDetentId,
          sheetHeight = with(density) { sheetHeightPx.toDp() },
          velocity = velocity,
        )
      ) {
        if (entry.spec.haptics.onDismiss) {
          haptics.perform(SheetHapticEvent.Dismiss)
        }
        entry.controller.dismiss(SheetDismissReason.Drag)
        return
      }

      val nearest = resolveSheetSettledDetent(
        policy = entry.spec.sizePolicy,
        detents = resolvedDetents,
        currentDetentId = entry.controller.currentDetentId,
        sheetHeight = with(density) { sheetHeightPx.toDp() },
        velocity = velocity,
      ) ?: return
      if (nearest.id == entry.controller.targetDetentId) {
        coroutineScope.launch {
          val animatable = Animatable(sheetHeightPx)
          animatable.animateTo(
            with(density) { nearest.height.toPx() },
            animationSpec = tween(
              durationMillis = SheetDefaults.HeightAnimationDuration,
              easing = SheetDefaults.EnterEasing,
            ),
          ) {
            sheetHeightPx = value
          }
          if (entry.spec.haptics.onDetentSnap && lastSettledDetentId != nearest.id) {
            haptics.perform(SheetHapticEvent.DetentSnap)
          }
          entry.controller.snapToCurrentTarget()
          lastSettledDetentId = nearest.id
        }
      } else {
        entry.controller.animateTo(nearest.id)
      }
    }

    val nestedScrollConnection = remember(
      entry,
      entry.isTopOfStack,
      minDetentHeightPx,
      maxDetentHeightPx,
    ) {
      object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
          if (
            !entry.isTopOfStack ||
            isResolving ||
            available.y >= 0f ||
            maxDetentHeightPx <= 0f
          ) {
            return Offset.Zero
          }
          val nextHeight = (sheetHeightPx - available.y).coerceAtMost(maxDetentHeightPx)
          val consumed = nextHeight - sheetHeightPx
          if (consumed <= 0f) return Offset.Zero
          sheetHeightPx = nextHeight
          return Offset(0f, -consumed)
        }

        override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
          if (!entry.isTopOfStack || isResolving || available.y <= 0f) {
            return Offset.Zero
          }
          val nextHeight = (sheetHeightPx - available.y).coerceAtLeast(0f)
          val consumedY = sheetHeightPx - nextHeight
          if (consumedY <= 0f) return Offset.Zero
          sheetHeightPx = nextHeight
          return Offset(0f, consumedY)
        }

        override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity {
          if (!entry.isTopOfStack || isResolving || resolvedDetents.isEmpty()) {
            return Velocity.Zero
          }
          settleOrDismiss(
            velocity = available.y.takeIf { it != 0f } ?: consumed.y,
          )
          return Velocity.Zero
        }
      }
    }

    val draggableState = rememberDraggableState { delta ->
      if (!entry.isTopOfStack || isResolving) return@rememberDraggableState
      sheetHeightPx = (sheetHeightPx - delta).coerceIn(0f, maxDetentHeightPx.coerceAtLeast(sheetHeightPx))
    }
    val dragRegionModifier = Modifier.draggable(
      state = draggableState,
      orientation = Orientation.Vertical,
      enabled = entry.isTopOfStack && !isResolving,
      onDragStopped = { velocity -> settleOrDismiss(velocity) },
    )

    PlatformBackHandler(
      enabled = !isResolving && entry.isTopOfStack && entry.spec.dismissPolicy.back && entry.mode == SheetMode.Modal,
    ) {
      entry.controller.dismiss(SheetDismissReason.Back)
    }

    val showScrim = entry.mode == SheetMode.Modal &&
      entry.isTopOfStack &&
      entry.spec.chrome.scrim.visible &&
      progress.value > 0f
    val scrimAlpha = progress.value * entry.spec.chrome.scrim.opacity
    val currentColors = AppTheme.colors

    Box(
      modifier = Modifier
        .fillMaxSize()
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
        val scrimModifier = Modifier
          .fillMaxSize()
          .alpha(scrimAlpha)
          .background(currentColors.scrim)

        if (!isResolving && entry.spec.chrome.scrim.blocksPointerInput && entry.spec.dismissPolicy.outsideTap) {
          Box(
            modifier = scrimModifier.clickable {
              entry.controller.dismiss(SheetDismissReason.OutsideTap)
            },
          )
        } else {
          Box(modifier = scrimModifier)
        }
      }

      ResponsiveContainer(
        modifier = Modifier.fillMaxWidth(),
        alignment = Alignment.BottomCenter,
      ) {
        Column(
          horizontalAlignment = Alignment.CenterHorizontally,
          modifier = Modifier
            .fillMaxWidth()
            .heightIn(max = maxSheetHeight)
            .then(
              if (renderedSheetHeightPx > 0f) {
                Modifier.height(with(density) { renderedSheetHeightPx.toDp() })
              } else {
                Modifier
              },
            )
            .offset {
              val animatedOffset = ((1f - progress.value) * renderedSheetHeightPx).roundToInt()
              IntOffset(0, animatedOffset)
            }
            .dropShadow(
              RoundedCornerShape(
                topStart = entry.spec.chrome.topCornerRadius,
                topEnd = entry.spec.chrome.topCornerRadius,
              ),
            ) {
              color = currentColors.shadowAmbient
              radius = 8f
            }
            .dropShadow(
              RoundedCornerShape(
                topStart = entry.spec.chrome.topCornerRadius,
                topEnd = entry.spec.chrome.topCornerRadius,
              ),
            ) {
              color = currentColors.shadow
              offset = Offset(0f, -4f)
              radius = 12f
            }
            .clip(
              RoundedCornerShape(
                topStart = entry.spec.chrome.topCornerRadius,
                topEnd = entry.spec.chrome.topCornerRadius,
              ),
            )
            .background(currentColors.surfaceRaised),
        ) {
          when (val handle = entry.spec.chrome.handle) {
            SheetHandleStyle.Hidden -> Unit
            is SheetHandleStyle.Visible -> {
              Box(
                modifier = Modifier
                  .then(dragRegionModifier)
                  .fillMaxWidth()
                  .height(handle.topPadding + handle.height + handle.bottomPadding),
                contentAlignment = Alignment.Center,
              ) {
                Box(
                  modifier = Modifier
                    .size(width = handle.width, height = handle.height)
                    .clip(RoundedCornerShape(handle.height / 2))
                    .background(currentColors.borderSubtle),
                )
              }
            }
          }

          Box(
            modifier = Modifier
              .fillMaxWidth()
              .weight(1f, fill = false)
              .nestedScroll(nestedScrollConnection),
          ) {
            CompositionLocalProvider(
              LocalViewModelStoreOwner provides viewModelStoreOwner,
              LocalSheetDragRegionModifier provides dragRegionModifier,
            ) {
              Column(
                modifier = Modifier
                  .fillMaxWidth()
                  .onSizeChanged { size ->
                    if (requiresContentMeasurement) {
                      measuredContentHeightPx = size.height.toFloat()
                    }
                  }
                  .alpha(if (isContentReady) 1f else 0f),
              ) {
                entry.content.invoke(sheetScope)
              }
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
): Float = when {
  requiresContentMeasurement -> 0f
  initialResolvedDetent != null -> with(density) { initialResolvedDetent.height.toPx() }
  else -> 0f
}

internal fun resolveRenderedSheetHeightPx(
  currentSheetHeightPx: Float,
  requiresContentMeasurement: Boolean,
  measuredContentHeightPx: Float,
  initialSheetHeightPx: Float,
): Float = when {
  currentSheetHeightPx > 0f -> currentSheetHeightPx
  requiresContentMeasurement -> measuredContentHeightPx
  else -> initialSheetHeightPx
}
