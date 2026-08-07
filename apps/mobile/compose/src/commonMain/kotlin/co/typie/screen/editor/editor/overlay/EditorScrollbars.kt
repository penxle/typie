package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.core.EaseInOutBack
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.AwaitPointerEventScope
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.body.resolvePaginatedPageGap
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportScrollbarMetrics
import co.typie.editor.viewport.EditorViewportState
import co.typie.editor.viewport.resolveEditorViewportScrollbarMetrics
import co.typie.editor.viewport.resolveEditorViewportScrollbarScrollPositionFromDrag
import co.typie.ext.LocalScrollGestureLockState
import co.typie.ext.ScrollGestureLockState
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeoutOrNull

private const val ScrollbarOpacityAnimationMs = 300
private const val ScrollbarThumbAnimationMs = 250
private const val ScrollbarMinThumbSize = 30f
private const val ScrollbarEdgePadding = 2f
private const val ScrollbarHitWidth = 20f
private const val ScrollbarThumbWidth = 6f
private const val ScrollbarActiveThumbWidth = 10f
private const val ScrollbarThumbLaneWidth = ScrollbarActiveThumbWidth + ScrollbarEdgePadding
private const val ScrollbarHideDelayMs = 1500L
private const val ScrollbarIndicatorHideDelayMs = 300L
private const val ScrollbarIndicatorHeight = 24f
private const val ScrollbarIndicatorGap = 8f
private val ScrollbarShape = AppShapes.rounded(4.dp)

internal const val EditorScrollbarPressAndHoldDurationMillis = 300L

internal enum class EditorScrollbarDragDecision {
  Pending,
  Claim,
  Yield,
}

internal fun resolveEditorScrollbarDragDecision(
  horizontal: Boolean,
  directDragEnabled: Boolean,
  displacement: Offset,
  touchSlop: Float,
  elapsedMillis: Long,
): EditorScrollbarDragDecision {
  val effectiveTouchSlop = touchSlop.coerceAtLeast(0f)
  val movedPastSlop = displacement.getDistanceSquared() > effectiveTouchSlop * effectiveTouchSlop
  if (movedPastSlop) {
    val axisDelta = abs(if (horizontal) displacement.x else displacement.y)
    val crossAxisDelta = abs(if (horizontal) displacement.y else displacement.x)
    if (axisDelta <= crossAxisDelta) {
      return EditorScrollbarDragDecision.Yield
    }

    return if (directDragEnabled) {
      EditorScrollbarDragDecision.Claim
    } else {
      EditorScrollbarDragDecision.Yield
    }
  }

  if (elapsedMillis >= EditorScrollbarPressAndHoldDurationMillis) {
    return if (directDragEnabled) {
      EditorScrollbarDragDecision.Claim
    } else {
      EditorScrollbarDragDecision.Yield
    }
  }

  return EditorScrollbarDragDecision.Pending
}

private data class EditorScrollbarLayoutMetrics(
  val viewportSize: Size,
  val contentSize: Size,
  val scrollOffset: Offset,
  val vertical: EditorViewportScrollbarMetrics,
  val horizontal: EditorViewportScrollbarMetrics,
)

private data class EditorScrollbarDragMetrics(
  val trackLength: Float,
  val thumbSize: Float,
  val viewportLength: Float,
  val contentLength: Float,
)

private fun resolveEditorScrollbarLayoutMetrics(
  viewportSize: Size,
  contentSize: Size,
  scrollOffset: Offset,
  visibleArea: EditorVisibleArea,
): EditorScrollbarLayoutMetrics {
  fun vertical(reserveHorizontalScrollbar: Boolean): EditorViewportScrollbarMetrics =
    resolveEditorViewportScrollbarMetrics(
      viewportLength = viewportSize.height,
      contentLength = contentSize.height,
      scrollPosition = scrollOffset.y,
      minThumbSize = ScrollbarMinThumbSize,
      leadingInset = visibleArea.topOcclusion,
      trailingInset = visibleArea.bottomOcclusion,
      leadingPadding = ScrollbarEdgePadding,
      trailingPadding =
        ScrollbarEdgePadding + if (reserveHorizontalScrollbar) ScrollbarThumbLaneWidth else 0f,
    )

  fun horizontal(reserveVerticalScrollbar: Boolean): EditorViewportScrollbarMetrics =
    resolveEditorViewportScrollbarMetrics(
      viewportLength = viewportSize.width,
      contentLength = contentSize.width,
      scrollPosition = scrollOffset.x,
      minThumbSize = ScrollbarMinThumbSize,
      leadingPadding = ScrollbarEdgePadding,
      trailingPadding =
        ScrollbarEdgePadding + if (reserveVerticalScrollbar) ScrollbarThumbLaneWidth else 0f,
    )

  val verticalWithoutHorizontalScrollbar = vertical(reserveHorizontalScrollbar = false)
  val horizontalWithoutVerticalScrollbar = horizontal(reserveVerticalScrollbar = false)
  val (vertical, horizontal) =
    when {
      !verticalWithoutHorizontalScrollbar.isVisible &&
        !horizontalWithoutVerticalScrollbar.isVisible ->
        verticalWithoutHorizontalScrollbar to horizontalWithoutVerticalScrollbar

      !verticalWithoutHorizontalScrollbar.isVisible ->
        vertical(reserveHorizontalScrollbar = true) to horizontalWithoutVerticalScrollbar

      !horizontalWithoutVerticalScrollbar.isVisible ->
        verticalWithoutHorizontalScrollbar to horizontal(reserveVerticalScrollbar = true)

      else -> {
        val verticalWithHorizontalScrollbar = vertical(reserveHorizontalScrollbar = true)
        val horizontalWithVerticalScrollbar = horizontal(reserveVerticalScrollbar = true)
        when {
          verticalWithHorizontalScrollbar.isVisible && horizontalWithVerticalScrollbar.isVisible ->
            verticalWithHorizontalScrollbar to horizontalWithVerticalScrollbar

          verticalWithHorizontalScrollbar.isVisible ->
            verticalWithoutHorizontalScrollbar to horizontalWithVerticalScrollbar

          horizontalWithVerticalScrollbar.isVisible ->
            verticalWithHorizontalScrollbar to horizontalWithoutVerticalScrollbar

          // Both axes cannot fit while reserving each other. Keep the vertical scrollbar, which is
          // the primary document-scrolling axis, and hide the horizontal one.
          else -> verticalWithoutHorizontalScrollbar to horizontalWithVerticalScrollbar
        }
      }
    }

  return EditorScrollbarLayoutMetrics(
    viewportSize = viewportSize,
    contentSize = contentSize,
    scrollOffset = scrollOffset,
    vertical = vertical,
    horizontal = horizontal,
  )
}

@Composable
internal fun EditorScrollbars(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  modifier: Modifier = Modifier,
  tryClaimDirectDrag: () -> Boolean = { true },
) {
  val haptic = LocalHapticFeedback.current
  var overlayVisible by remember { mutableStateOf(false) }
  var indicatorVisible by remember { mutableStateOf(false) }
  var lastScrollWasAuto by remember { mutableStateOf(false) }
  var isVerticalThumbDragged by remember { mutableStateOf(false) }
  var isHorizontalThumbDragged by remember { mutableStateOf(false) }
  var hideSequenceRevision by remember { mutableIntStateOf(0) }

  val isDragging = isVerticalThumbDragged || isHorizontalThumbDragged
  val directDragEnabled = overlayVisible || isDragging

  DisposableEffect(viewportState) {
    onDispose { viewportState.updateScrollbarDragInProgress(false) }
  }
  LaunchedEffect(viewportState, isDragging) {
    viewportState.updateScrollbarDragInProgress(isDragging)
  }
  LaunchedEffect(viewportState) {
    snapshotFlow { viewportState.lastScrollRevision to viewportState.lastScrollWasAuto }
      .collect { (revision, isAutoScroll) ->
        if (revision <= 0) {
          return@collect
        }

        overlayVisible = true
        indicatorVisible = !isAutoScroll
        lastScrollWasAuto = isAutoScroll
        if (!isVerticalThumbDragged && !isHorizontalThumbDragged) {
          hideSequenceRevision += 1
        }
      }
  }
  LaunchedEffect(hideSequenceRevision, isDragging, overlayVisible) {
    if (isDragging || !overlayVisible || hideSequenceRevision <= 0) {
      return@LaunchedEffect
    }

    val revision = hideSequenceRevision
    delay(ScrollbarIndicatorHideDelayMs)
    if (revision == hideSequenceRevision) {
      indicatorVisible = false
    }
    delay(ScrollbarHideDelayMs - ScrollbarIndicatorHideDelayMs)
    if (revision == hideSequenceRevision) {
      overlayVisible = false
    }
  }
  val overlayAlpha by
    animateFloatAsState(
      targetValue =
        resolveEditorScrollbarOpacity(
          visible = overlayVisible,
          dragging = isDragging,
          isAutoScroll = lastScrollWasAuto,
        ),
      animationSpec = tween(durationMillis = ScrollbarOpacityAnimationMs, easing = LinearEasing),
      label = "editor-scrollbar-overlay-alpha",
    )
  val indicatorAlpha by
    animateFloatAsState(
      targetValue =
        if (!lastScrollWasAuto && (indicatorVisible || isVerticalThumbDragged)) {
          1f
        } else {
          0f
        },
      animationSpec = tween(durationMillis = ScrollbarOpacityAnimationMs, easing = LinearEasing),
      label = "editor-scrollbar-indicator-alpha",
    )
  val indicatorText =
    resolveEditorScrollbarIndicatorText(
      layoutSpec = layoutSpec,
      pageSizes = pageSizes,
      displayZoom = displayZoom,
      viewportLength = viewportState.viewportSize.height,
      contentLength = viewportState.contentSize.height,
      scrollPosition = viewportState.scrollOffset.y,
    )

  Box(modifier = modifier.fillMaxSize()) {
    Box(modifier = Modifier.fillMaxSize().graphicsLayer { alpha = overlayAlpha }) {
      EditorScrollbarThumb(
        viewportState = viewportState,
        visibleArea = visibleArea,
        isAutoScroll = lastScrollWasAuto,
        directDragEnabled = directDragEnabled,
        isDragging = isVerticalThumbDragged,
        tryClaimDirectDrag = tryClaimDirectDrag,
        onDragChanged = { dragging ->
          if (isVerticalThumbDragged != dragging) {
            haptic.performHapticFeedback(
              if (dragging) {
                HapticFeedbackType.GestureThresholdActivate
              } else {
                HapticFeedbackType.GestureEnd
              }
            )
          }
          isVerticalThumbDragged = dragging
          overlayVisible = true
          indicatorVisible = true
          lastScrollWasAuto = false
          if (!dragging && !isHorizontalThumbDragged) {
            hideSequenceRevision += 1
          }
        },
      )
      EditorScrollbarThumb(
        horizontal = true,
        viewportState = viewportState,
        visibleArea = visibleArea,
        isAutoScroll = lastScrollWasAuto,
        directDragEnabled = directDragEnabled,
        isDragging = isHorizontalThumbDragged,
        tryClaimDirectDrag = tryClaimDirectDrag,
        onDragChanged = { dragging ->
          if (isHorizontalThumbDragged != dragging) {
            haptic.performHapticFeedback(
              if (dragging) {
                HapticFeedbackType.GestureThresholdActivate
              } else {
                HapticFeedbackType.GestureEnd
              }
            )
          }
          isHorizontalThumbDragged = dragging
          overlayVisible = true
          indicatorVisible = true
          lastScrollWasAuto = false
          if (!dragging && !isVerticalThumbDragged) {
            hideSequenceRevision += 1
          }
        },
      )
    }

    if (
      indicatorText != null && (indicatorAlpha > 0f || indicatorVisible || isVerticalThumbDragged)
    ) {
      EditorScrollbarIndicator(
        viewportState = viewportState,
        visibleArea = visibleArea,
        text = indicatorText,
        alpha = indicatorAlpha,
      )
    }
  }
}

internal fun resolveEditorScrollbarOpacity(
  visible: Boolean,
  dragging: Boolean,
  isAutoScroll: Boolean,
): Float =
  if (visible || dragging) {
    if (isAutoScroll) 0.65f else 1f
  } else {
    0f
  }

internal fun resolveEditorScrollbarThumbThickness(dragging: Boolean): Float =
  if (dragging) ScrollbarActiveThumbWidth else ScrollbarThumbWidth

internal fun resolveEditorScrollbarThumbAlpha(dragging: Boolean, isAutoScroll: Boolean): Float =
  if (isAutoScroll) {
    if (dragging) 0.45f else 0.22f
  } else {
    if (dragging) 0.8f else 0.5f
  }

internal fun resolveEditorScrollbarIndicatorText(
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  viewportLength: Float,
  contentLength: Float,
  scrollPosition: Float,
): String? =
  when (layoutSpec) {
    is EditorDocumentLayoutSpec.Paginated ->
      resolveEditorScrollbarMostVisiblePage(
          pageSizes = pageSizes,
          displayZoom = displayZoom,
          viewportLength = viewportLength,
          scrollPosition = scrollPosition,
        )
        ?.let { page -> "${page + 1}/${pageSizes.size}" }

    is EditorDocumentLayoutSpec.Continuous -> {
      val maxScroll = (contentLength - viewportLength).coerceAtLeast(0f)
      val ratio = if (maxScroll > 0f) (scrollPosition / maxScroll).coerceIn(0f, 1f) else 0f
      "${(ratio * 100f).roundToInt()}%"
    }
  }

private fun resolveEditorScrollbarMostVisiblePage(
  pageSizes: List<PageSize>,
  displayZoom: Float,
  viewportLength: Float,
  scrollPosition: Float,
): Int? {
  if (pageSizes.isEmpty()) {
    return null
  }

  val effectiveZoom =
    if (displayZoom.isFinite() && displayZoom > 0f) {
      displayZoom
    } else {
      1f
    }
  val viewportTop = scrollPosition.coerceAtLeast(0f)
  val viewportBottom = viewportTop + viewportLength.coerceAtLeast(0f)
  var pageTop = 0f
  var mostVisiblePage = 0
  var maxVisibleLength = 0f

  pageSizes.forEachIndexed { index, size ->
    val pageHeight = (size.height * effectiveZoom).coerceAtLeast(0f)
    val pageBottom = pageTop + pageHeight
    val visibleTop = max(pageTop, viewportTop)
    val visibleBottom = min(pageBottom, viewportBottom)
    val visibleLength = (visibleBottom - visibleTop).coerceAtLeast(0f)

    if (visibleLength > maxVisibleLength) {
      maxVisibleLength = visibleLength
      mostVisiblePage = index
    }

    pageTop =
      pageBottom + if (index < pageSizes.lastIndex) resolvePaginatedPageGap(effectiveZoom) else 0f
  }

  return mostVisiblePage
}

@Composable
private fun EditorScrollbarIndicator(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  text: String,
  alpha: Float,
) {
  EditorScrollbarIndicatorLayout(
    viewportState = viewportState,
    visibleArea = visibleArea,
    modifier = Modifier.fillMaxSize(),
  ) {
    Box(
      modifier =
        Modifier.graphicsLayer { this.alpha = alpha }
          .height(ScrollbarIndicatorHeight.dp)
          .background(AppTheme.colors.surfaceInverse.copy(alpha = 0.65f), ScrollbarShape)
          .padding(horizontal = 8.dp, vertical = 4.dp),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = text,
        style = TextStyle(fontSize = 11.sp, fontFeatureSettings = "tnum"),
        color = AppTheme.colors.textOnInverse,
        maxLines = 1,
      )
    }
  }
}

@Composable
internal fun EditorScrollbarIndicatorLayout(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  Layout(modifier = modifier, content = content) { measurables, constraints ->
    val layoutMetrics = resolveEditorScrollbarLayoutMetrics(viewportState, visibleArea)
    val verticalMetrics = layoutMetrics.vertical
    val placeable = measurables.single().measure(constraints.copy(minWidth = 0, minHeight = 0))

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      if (verticalMetrics.isVisible) {
        val top =
          visibleArea.topOcclusion +
            ScrollbarEdgePadding +
            verticalMetrics.thumbOffset +
            verticalMetrics.thumbSize / 2f - ScrollbarIndicatorHeight / 2f
        val right = ScrollbarThumbLaneWidth + ScrollbarIndicatorGap
        val x =
          (Dp(layoutMetrics.viewportSize.width - right).roundToPx() - placeable.width)
            .coerceAtLeast(0)
        val y = Dp(top).roundToPx().coerceAtLeast(0)
        placeable.place(x = x, y = y)
      }
    }
  }
}

@Composable
private fun EditorScrollbarThumb(
  horizontal: Boolean = false,
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  isAutoScroll: Boolean,
  directDragEnabled: Boolean,
  isDragging: Boolean,
  tryClaimDirectDrag: () -> Boolean,
  onDragChanged: (Boolean) -> Unit,
) {
  val density = LocalDensity.current
  val scrollGestureLockState = LocalScrollGestureLockState.current
  val latestDirectDragEnabled = rememberUpdatedState(directDragEnabled)
  val latestVisibleArea = rememberUpdatedState(visibleArea)
  val latestTryClaimDirectDrag = rememberUpdatedState(tryClaimDirectDrag)
  val latestOnDragChanged = rememberUpdatedState(onDragChanged)
  val thumbLaneWidth = ScrollbarThumbLaneWidth.dp
  val edgePadding = ScrollbarEdgePadding.dp
  val animatedThumbThickness by
    animateDpAsState(
      targetValue = Dp(resolveEditorScrollbarThumbThickness(isDragging)),
      animationSpec = tween(durationMillis = ScrollbarThumbAnimationMs, easing = EaseInOutBack),
      label = "editor-scrollbar-thumb-thickness",
    )
  val animatedThumbAlpha by
    animateFloatAsState(
      targetValue =
        resolveEditorScrollbarThumbAlpha(dragging = isDragging, isAutoScroll = isAutoScroll),
      animationSpec = tween(durationMillis = ScrollbarThumbAnimationMs, easing = EaseInOutBack),
      label = "editor-scrollbar-thumb-alpha",
    )

  EditorScrollbarThumbLayout(
    horizontal = horizontal,
    viewportState = viewportState,
    visibleArea = visibleArea,
    modifier = Modifier.fillMaxSize(),
  ) {
    Box(
      modifier =
        Modifier.fillMaxSize().pointerInput(
          horizontal,
          viewportState,
          density,
          scrollGestureLockState,
        ) {
          awaitEachGesture {
            val admission =
              awaitEditorScrollbarDragAdmission(
                horizontal = horizontal,
                directDragEnabled = { latestDirectDragEnabled.value },
                tryClaimDirectDrag = { latestTryClaimDirectDrag.value() },
                touchSlop = viewConfiguration.touchSlop,
              ) ?: return@awaitEachGesture
            dragEditorScrollbar(
              admission = admission,
              horizontal = horizontal,
              density = density,
              viewportState = viewportState,
              scrollGestureLockState = scrollGestureLockState,
              currentLayoutMetrics = {
                resolveEditorScrollbarLayoutMetrics(viewportState, latestVisibleArea.value)
              },
              onDragChanged = { latestOnDragChanged.value(it) },
            )
          }
        }
    ) {
      if (!horizontal) {
        Box(
          modifier =
            Modifier.align(Alignment.CenterEnd)
              .width(thumbLaneWidth)
              .fillMaxHeight()
              .padding(end = edgePadding),
          contentAlignment = Alignment.CenterEnd,
        ) {
          EditorScrollbarThumbBody(
            modifier = Modifier.width(animatedThumbThickness).fillMaxHeight(),
            alpha = animatedThumbAlpha,
          )
        }
      } else {
        Box(
          modifier =
            Modifier.align(Alignment.BottomCenter)
              .fillMaxWidth()
              .height(thumbLaneWidth)
              .padding(bottom = edgePadding),
          contentAlignment = Alignment.BottomCenter,
        ) {
          EditorScrollbarThumbBody(
            modifier = Modifier.fillMaxWidth().height(animatedThumbThickness),
            alpha = animatedThumbAlpha,
          )
        }
      }
    }
  }
}

@Composable
internal fun EditorScrollbarThumbLayout(
  horizontal: Boolean,
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  modifier: Modifier = Modifier,
  content: @Composable () -> Unit,
) {
  Layout(modifier = modifier, content = content) { measurables, constraints ->
    val layoutMetrics = resolveEditorScrollbarLayoutMetrics(viewportState, visibleArea)
    val axisMetrics = if (!horizontal) layoutMetrics.vertical else layoutMetrics.horizontal
    val hitThickness = Dp(ScrollbarHitWidth).roundToPx()
    val thumbLength = Dp(axisMetrics.thumbSize).roundToPx()
    val childConstraints =
      if (!horizontal) {
        Constraints.fixed(width = hitThickness, height = thumbLength)
      } else {
        Constraints.fixed(width = thumbLength, height = hitThickness)
      }
    val placeable = measurables.single().measure(childConstraints)

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      if (axisMetrics.isVisible) {
        val x =
          if (!horizontal) {
            Dp(layoutMetrics.viewportSize.width - ScrollbarHitWidth).roundToPx().coerceAtLeast(0)
          } else {
            Dp(ScrollbarEdgePadding + axisMetrics.thumbOffset).roundToPx().coerceAtLeast(0)
          }
        val y =
          if (!horizontal) {
            Dp(visibleArea.topOcclusion + ScrollbarEdgePadding + axisMetrics.thumbOffset)
              .roundToPx()
              .coerceAtLeast(0)
          } else {
            Dp(layoutMetrics.viewportSize.height - visibleArea.bottomOcclusion - ScrollbarHitWidth)
              .roundToPx()
              .coerceAtLeast(0)
          }
        placeable.place(x = x, y = y)
      }
    }
  }
}

private data class EditorScrollbarDragAdmission(
  val down: PointerInputChange,
  val initialDragAmount: Offset,
)

private sealed interface EditorScrollbarPendingResult {
  data class Claim(val change: PointerInputChange) : EditorScrollbarPendingResult

  data object Yield : EditorScrollbarPendingResult
}

private suspend fun AwaitPointerEventScope.awaitEditorScrollbarDragAdmission(
  horizontal: Boolean,
  directDragEnabled: () -> Boolean,
  tryClaimDirectDrag: () -> Boolean,
  touchSlop: Float,
): EditorScrollbarDragAdmission? {
  val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
  val downInActualBounds =
    down.position.x >= 0f &&
      down.position.x < size.width.toFloat() &&
      down.position.y >= 0f &&
      down.position.y < size.height.toFloat()
  if (down.isConsumed || !downInActualBounds) {
    return null
  }

  var latestDisplacement = Offset.Zero
  val pendingResult =
    withTimeoutOrNull<EditorScrollbarPendingResult>(
      timeMillis = EditorScrollbarPressAndHoldDurationMillis
    ) {
      while (true) {
        val event = awaitPointerEvent(PointerEventPass.Initial)
        val change =
          event.changes.firstOrNull { it.id == down.id }
            ?: return@withTimeoutOrNull EditorScrollbarPendingResult.Yield
        if (!change.pressed || change.isConsumed) {
          return@withTimeoutOrNull EditorScrollbarPendingResult.Yield
        }

        latestDisplacement = change.position - down.position
        when (
          resolveEditorScrollbarDragDecision(
            horizontal = horizontal,
            directDragEnabled = directDragEnabled(),
            displacement = latestDisplacement,
            touchSlop = touchSlop,
            elapsedMillis = (change.uptimeMillis - down.uptimeMillis).coerceAtLeast(0L),
          )
        ) {
          EditorScrollbarDragDecision.Pending -> {
            val finalEvent = awaitPointerEvent(PointerEventPass.Final)
            val finalChange =
              finalEvent.changes.firstOrNull { it.id == down.id }
                ?: return@withTimeoutOrNull EditorScrollbarPendingResult.Yield
            if (!finalChange.pressed || finalChange.isConsumed) {
              return@withTimeoutOrNull EditorScrollbarPendingResult.Yield
            }
          }

          EditorScrollbarDragDecision.Claim ->
            return@withTimeoutOrNull EditorScrollbarPendingResult.Claim(change)

          EditorScrollbarDragDecision.Yield ->
            return@withTimeoutOrNull EditorScrollbarPendingResult.Yield
        }
      }
      error("Unreachable scrollbar admission state")
    }

  val claimedChange =
    when (pendingResult) {
      null -> {
        val timeoutDecision =
          resolveEditorScrollbarDragDecision(
            horizontal = horizontal,
            directDragEnabled = directDragEnabled(),
            displacement = latestDisplacement,
            touchSlop = touchSlop,
            elapsedMillis = EditorScrollbarPressAndHoldDurationMillis,
          )
        if (timeoutDecision != EditorScrollbarDragDecision.Claim) {
          return null
        }
        null
      }

      is EditorScrollbarPendingResult.Claim -> pendingResult.change
      EditorScrollbarPendingResult.Yield -> return null
    }
  val initialDragAmount = claimedChange?.positionChange() ?: Offset.Zero
  if (!tryClaimDirectDrag()) {
    return null
  }
  claimedChange?.consume()
  return EditorScrollbarDragAdmission(down = down, initialDragAmount = initialDragAmount)
}

private suspend fun AwaitPointerEventScope.dragEditorScrollbar(
  admission: EditorScrollbarDragAdmission,
  horizontal: Boolean,
  density: Density,
  viewportState: EditorViewportState,
  scrollGestureLockState: ScrollGestureLockState,
  currentLayoutMetrics: () -> EditorScrollbarLayoutMetrics,
  onDragChanged: (Boolean) -> Unit,
) {
  val dragLock = scrollGestureLockState.acquire()
  var dragActivated = false
  try {
    val dragStartLayoutMetrics = currentLayoutMetrics()
    var dragMetrics = dragStartLayoutMetrics.dragMetrics(horizontal)
    var dragStartScroll = dragStartLayoutMetrics.scrollPosition(horizontal)
    var accumulatedDrag = 0f

    fun applyDragAmount(dragAmount: Offset) {
      val dragDelta =
        with(density) {
          if (!horizontal) {
            dragAmount.y.toDp().value
          } else {
            dragAmount.x.toDp().value
          }
        }
      if (dragDelta == 0f) {
        return
      }

      val latestLayoutMetrics = currentLayoutMetrics()
      val latestDragMetrics = latestLayoutMetrics.dragMetrics(horizontal)
      if (latestDragMetrics != dragMetrics) {
        dragMetrics = latestDragMetrics
        dragStartScroll = latestLayoutMetrics.scrollPosition(horizontal)
        accumulatedDrag = 0f
      }
      val nextScrollPosition =
        resolveEditorViewportScrollbarScrollPositionFromDrag(
          startScrollPosition = dragStartScroll,
          dragDelta = accumulatedDrag + dragDelta,
          trackLength = dragMetrics.trackLength,
          thumbSize = dragMetrics.thumbSize,
          viewportLength = dragMetrics.viewportLength,
          contentLength = dragMetrics.contentLength,
        )

      accumulatedDrag += dragDelta
      val currentScrollOffset = viewportState.scrollOffset
      viewportState.scrollTo(
        offset =
          if (!horizontal) {
            Offset(x = currentScrollOffset.x, y = nextScrollPosition)
          } else {
            Offset(x = nextScrollPosition, y = currentScrollOffset.y)
          }
      )
    }

    onDragChanged(true)
    dragActivated = true
    applyDragAmount(admission.initialDragAmount)
    while (true) {
      val event = awaitPointerEvent(PointerEventPass.Initial)
      val change = event.changes.firstOrNull { it.id == admission.down.id } ?: break
      if (!change.pressed) {
        change.consume()
        break
      }

      val dragAmount = change.positionChange()
      if (dragAmount != Offset.Zero) {
        change.consume()
        applyDragAmount(dragAmount)
      }
    }
  } finally {
    try {
      if (dragActivated) {
        onDragChanged(false)
      }
    } finally {
      dragLock.release()
    }
  }
}

private fun EditorScrollbarLayoutMetrics.dragMetrics(
  horizontal: Boolean
): EditorScrollbarDragMetrics {
  val axisMetrics = if (!horizontal) vertical else this.horizontal
  return EditorScrollbarDragMetrics(
    trackLength = axisMetrics.trackLength,
    thumbSize = axisMetrics.thumbSize,
    viewportLength = if (!horizontal) viewportSize.height else viewportSize.width,
    contentLength = if (!horizontal) contentSize.height else contentSize.width,
  )
}

private fun EditorScrollbarLayoutMetrics.scrollPosition(horizontal: Boolean): Float =
  if (!horizontal) scrollOffset.y else scrollOffset.x

private fun resolveEditorScrollbarLayoutMetrics(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
): EditorScrollbarLayoutMetrics =
  resolveEditorScrollbarLayoutMetrics(
    viewportSize = viewportState.viewportSize,
    contentSize = viewportState.contentSize,
    scrollOffset = viewportState.scrollOffset,
    visibleArea = visibleArea,
  )

@Composable
private fun EditorScrollbarThumbBody(modifier: Modifier, alpha: Float) {
  Box(
    modifier =
      modifier.clip(ScrollbarShape).background(AppTheme.colors.surfaceInverse.copy(alpha = alpha))
  )
}
