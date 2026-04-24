package co.typie.ui.component.popover

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
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
import co.typie.ext.EdgeAutoScrollController
import co.typie.ext.edgeAutoScroll
import co.typie.ext.rememberEdgeAutoScrollController
import co.typie.ext.toDp
import co.typie.ext.toPx
import co.typie.ext.verticalScroll
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

data class PopoverPaneTransition(val progress: Float, val anchorContentRect: Rect)

val LocalPopoverPaneTransition = staticCompositionLocalOf<PopoverPaneTransition?> { null }
val LocalPopoverPaneEdgeAutoScrollController =
  staticCompositionLocalOf<EdgeAutoScrollController?> { null }

internal enum class PopoverPaneRenderPhase {
  Measure,
  Interactive,
}

internal val LocalPopoverPaneRenderPhase = staticCompositionLocalOf {
  PopoverPaneRenderPhase.Interactive
}

private enum class PopoverPaneSlot {
  InitialMeasurePane,
  FinalMeasurePane,
  Surface,
}

@Composable
fun PopoverOverlay(state: PopoverOverlayState) {
  val entry = state.entry ?: return
  val detachedCloseProgress = remember { Animatable(0f) }
  val detachedCloseRequestId = state.detachedCloseRequestId()

  LaunchedEffect(state.isDetached, detachedCloseRequestId, entry) {
    if (!state.isDetached) {
      detachedCloseProgress.snapTo(0f)
      return@LaunchedEffect
    }

    val from = state.progress.takeIf { it > 0f } ?: 1f
    detachedCloseProgress.stop()
    detachedCloseProgress.snapTo(from)
    detachedCloseProgress.animateTo(
      targetValue = 0f,
      animationSpec = tween(PopoverDefaults.ReverseDuration, easing = LinearEasing),
    ) {
      state.updateDetachedProgress(detachedCloseRequestId, value)
    }
    state.clearDetached(detachedCloseRequestId)
  }

  val progress = state.easedProgress
  if (progress <= 0f) return

  PopoverPaneContent(
    anchor = entry.anchor,
    pane = entry.pane,
    anchorBounds = state.anchorBounds,
    placement = entry.placement,
    progress = progress,
    interactive = state.interactive,
    collapsedCornerRadius = entry.collapsedCornerRadius,
    screenPadding = entry.screenPadding,
    maxWidth = entry.maxWidth,
    minWidth = entry.minWidth,
    expandToMaxWidth = entry.expandToMaxWidth,
    onPaneBoundsChanged = { state.updatePaneBounds(entry.owner, it) },
  )
}

@Composable
private fun PopoverPaneContent(
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
  onPaneBoundsChanged: (Rect) -> Unit,
) {
  val density = LocalDensity.current
  var layoutPositionInWindow by remember { mutableStateOf(IntOffset.Zero) }

  SubcomposeLayout(
    modifier =
      Modifier.onGloballyPositioned { coordinates ->
        layoutPositionInWindow = coordinates.positionInWindow().round()
      }
  ) { constraints ->
    val anchorBoundsInLayout =
      localizePopoverAnchorBounds(
        anchorBounds = anchorBounds,
        layoutPositionInWindow = layoutPositionInWindow,
      )
    val minWidthPx = minWidth.toPx(density).roundToInt()
    val maxWidthPx = maxWidth?.toPx(density)?.roundToInt()
    val preferredPaneMaxWidth =
      availableWidthForPlacement(
        windowWidth = constraints.maxWidth,
        anchorBounds = anchorBoundsInLayout,
        screenPadding = screenPadding,
        placement = placement,
      )
    val paneMaxWidth =
      min(
          shrinkBounded(constraints.maxWidth, screenPadding.left + screenPadding.right),
          preferredPaneMaxWidth,
        )
        .let { w -> if (maxWidthPx != null) min(w, maxWidthPx) else w }
    val paneConstraints =
      constraints.copy(
        minWidth = 0,
        minHeight = 0,
        maxWidth = paneMaxWidth,
        maxHeight = shrinkBounded(constraints.maxHeight, screenPadding.top + screenPadding.bottom),
      )

    val initialPanePlaceables =
      subcompose(PopoverPaneSlot.InitialMeasurePane) {
          CompositionLocalProvider(
            LocalPopoverPaneRenderPhase provides PopoverPaneRenderPhase.Measure
          ) {
            ShrinkWrappedPane(expandToMaxWidth = expandToMaxWidth, content = pane)
          }
        }
        .map { it.measure(paneConstraints) }

    val initiallyMeasuredWidth =
      initialPanePlaceables.maxOfOrNull { it.width } ?: anchorBoundsInLayout.width
    val initiallyMeasuredHeight =
      initialPanePlaceables.maxOfOrNull { it.height } ?: anchorBoundsInLayout.height
    val showBelow =
      shouldShowBelow(
        placement = placement,
        childHeight = initiallyMeasuredHeight,
        windowHeight = constraints.maxHeight,
        anchorRect = anchorBoundsInLayout,
        screenPadding = screenPadding,
      )
    val finalPaneConstraints =
      paneConstraints.copy(
        maxHeight =
          availableHeightForPlacement(
            windowHeight = constraints.maxHeight,
            anchorBounds = anchorBoundsInLayout,
            screenPadding = screenPadding,
            showBelow = showBelow,
          )
      )
    val finalPanePlaceables =
      subcompose(PopoverPaneSlot.FinalMeasurePane) {
          CompositionLocalProvider(
            LocalPopoverPaneRenderPhase provides PopoverPaneRenderPhase.Measure
          ) {
            ShrinkWrappedPane(expandToMaxWidth = expandToMaxWidth, content = pane)
          }
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
        anchorBounds = anchorBoundsInLayout,
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
      subcompose(PopoverPaneSlot.Surface) {
          Box(
            modifier =
              Modifier.onGloballyPositioned { coordinates ->
                val pos = coordinates.positionInWindow()
                onPaneBoundsChanged(
                  Rect(
                    left = pos.x,
                    top = pos.y,
                    right = pos.x + coordinates.size.width,
                    bottom = pos.y + coordinates.size.height,
                  )
                )
              }
          ) {
            CompositionLocalProvider(
              LocalPopoverPaneTransition provides transition,
              LocalPopoverPaneRenderPhase provides PopoverPaneRenderPhase.Interactive,
            ) {
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
        }
        .single()
        .measure(Constraints.fixed(resolvedPaneWidth, paneHeight))

    layout(constraints.maxWidth, constraints.maxHeight) {
      surfacePlaceable.place(geometry.popupOffset)
    }
  }
}

@Composable
private fun ShrinkWrappedPane(expandToMaxWidth: Boolean = false, content: @Composable () -> Unit) {
  val scrollState = rememberScrollState()
  val controller = rememberEdgeAutoScrollController(verticalScrollableState = scrollState)

  CompositionLocalProvider(LocalPopoverPaneEdgeAutoScrollController provides controller) {
    Box(
      modifier =
        Modifier.then(
            if (expandToMaxWidth) {
              Modifier.fillMaxWidth()
            } else {
              Modifier.width(IntrinsicSize.Max)
            }
          )
          .edgeAutoScroll(controller)
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
  val surfaceOffset = surfaceOffsetForProgress(anchorContentRect, progress)
  val paneOffset = IntOffset(x = -surfaceOffset.x, y = -surfaceOffset.y)
  val anchorOffset = IntOffset(x = anchorContentRect.left, y = anchorContentRect.top)
  val cornerRadius =
    lerp(
      collapsedCornerRadius.toPx(density),
      PopoverDefaults.ExpandedRadius.toPx(density),
      progress,
    )
  val paneShape = AppShapes.squircle(cornerRadius.toDp(density))

  Box(
    modifier =
      Modifier.size(width = paneSize.width.toDp(density), height = paneSize.height.toDp(density))
  ) {
    Box(
      modifier =
        Modifier.offset { surfaceOffset }
          .size(width = animatedWidth.toDp(density), height = animatedHeight.toDp(density))
          .shadow(AppTheme.shadows.md, paneShape, alpha = { progress })
          .border(1.dp, AppTheme.colors.borderDefault, paneShape)
          .background(AppTheme.colors.surfaceDefault, paneShape)
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
        pane = { Box(modifier = Modifier.graphicsLayer { alpha = progress }) { pane() } },
        anchor = { Box(modifier = Modifier.graphicsLayer { alpha = 1f - progress }) { anchor() } },
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
