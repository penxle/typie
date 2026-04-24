package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.core.EaseInOutBack
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.TextStyle
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
import co.typie.ui.component.Text
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt
import kotlinx.coroutines.delay

private const val ScrollbarOpacityAnimationMs = 300
private const val ScrollbarThumbAnimationMs = 250
private const val ScrollbarMinThumbSize = 30f
private const val ScrollbarTrackPadding = 2f
private const val ScrollbarTrackWidth = 12f
private const val ScrollbarThumbWidth = 6f
private const val ScrollbarActiveThumbWidth = 10f
private const val ScrollbarLongPressHitExpansion = 16f
private const val ScrollbarHideDelayMs = 1500L
private const val ScrollbarIndicatorHideDelayMs = 300L
private const val ScrollbarIndicatorHeight = 24f
private const val ScrollbarIndicatorGap = 8f
private val ScrollbarShape = AppShapes.rounded(4.dp)

@Composable
internal fun EditorScrollbars(
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  layoutSpec: EditorDocumentLayoutSpec,
  pageSizes: List<PageSize>,
  displayZoom: Float,
  modifier: Modifier = Modifier,
) {
  val haptic = LocalHapticFeedback.current
  val viewportSize = viewportState.viewportSize
  val contentSize = viewportState.contentSize
  val scrollOffset = viewportState.scrollOffset
  val hasVerticalScroll = contentSize.height > viewportSize.height
  val hasHorizontalScroll = contentSize.width > viewportSize.width

  val verticalMetrics =
    resolveEditorViewportScrollbarMetrics(
      viewportLength = viewportSize.height,
      contentLength = contentSize.height,
      scrollPosition = scrollOffset.y,
      minThumbSize = ScrollbarMinThumbSize,
      leadingInset = visibleArea.topOcclusion,
      trailingInset = visibleArea.bottomOcclusion,
      leadingPadding = ScrollbarTrackPadding,
      trailingPadding = ScrollbarTrackPadding + if (hasHorizontalScroll) ScrollbarTrackWidth else 0f,
    )
  val horizontalMetrics =
    resolveEditorViewportScrollbarMetrics(
      viewportLength = viewportSize.width,
      contentLength = contentSize.width,
      scrollPosition = scrollOffset.x,
      minThumbSize = ScrollbarMinThumbSize,
      leadingPadding = ScrollbarTrackPadding,
      trailingPadding = ScrollbarTrackPadding + if (hasVerticalScroll) ScrollbarTrackWidth else 0f,
    )
  val hasVisibleScrollbar = verticalMetrics.isVisible || horizontalMetrics.isVisible
  var overlayVisible by remember { mutableStateOf(false) }
  var indicatorVisible by remember { mutableStateOf(false) }
  var wasLastScrollUser by remember { mutableStateOf(false) }
  var isVerticalThumbDragged by remember { mutableStateOf(false) }
  var isHorizontalThumbDragged by remember { mutableStateOf(false) }
  var hideSequenceRevision by remember { mutableIntStateOf(0) }

  val isDragging = isVerticalThumbDragged || isHorizontalThumbDragged
  val inputEnabled = hasVisibleScrollbar && (overlayVisible || isDragging)

  DisposableEffect(viewportState) {
    onDispose { viewportState.updateScrollbarDragInProgress(false) }
  }
  LaunchedEffect(viewportState, isDragging) {
    viewportState.updateScrollbarDragInProgress(isDragging)
  }
  LaunchedEffect(hasVisibleScrollbar) {
    if (!hasVisibleScrollbar) {
      overlayVisible = false
      indicatorVisible = false
      wasLastScrollUser = false
      isVerticalThumbDragged = false
      isHorizontalThumbDragged = false
      hideSequenceRevision = 0
    }
  }
  LaunchedEffect(viewportState, hasVisibleScrollbar) {
    snapshotFlow { viewportState.lastScrollRevision to viewportState.wasLastScrollUser }
      .collect { (revision, isUserScroll) ->
        if (revision <= 0 || !hasVisibleScrollbar) {
          return@collect
        }

        overlayVisible = true
        indicatorVisible = isUserScroll
        wasLastScrollUser = isUserScroll
        if (!isVerticalThumbDragged && !isHorizontalThumbDragged) {
          hideSequenceRevision += 1
        }
      }
  }
  LaunchedEffect(hasVisibleScrollbar, hideSequenceRevision, isDragging, overlayVisible) {
    if (!hasVisibleScrollbar || isDragging || !overlayVisible || hideSequenceRevision <= 0) {
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
          isUserScroll = wasLastScrollUser,
        ),
      animationSpec = tween(durationMillis = ScrollbarOpacityAnimationMs, easing = LinearEasing),
      label = "editor-scrollbar-overlay-alpha",
    )
  val indicatorAlpha by
    animateFloatAsState(
      targetValue =
        if (
          verticalMetrics.isVisible &&
            wasLastScrollUser &&
            (indicatorVisible || isVerticalThumbDragged)
        ) {
          1f
        } else {
          0f
        },
      animationSpec = tween(durationMillis = ScrollbarOpacityAnimationMs, easing = LinearEasing),
      label = "editor-scrollbar-indicator-alpha",
    )
  val shouldComposeThumbs = hasVisibleScrollbar && (inputEnabled || overlayAlpha > 0f)
  val indicatorText =
    if (verticalMetrics.isVisible) {
      resolveEditorScrollbarIndicatorText(
        layoutSpec = layoutSpec,
        pageSizes = pageSizes,
        displayZoom = displayZoom,
        viewportLength = viewportSize.height,
        contentLength = contentSize.height,
        scrollPosition = scrollOffset.y,
      )
    } else {
      null
    }

  Box(modifier = modifier.fillMaxSize()) {
    Box(modifier = Modifier.fillMaxSize().graphicsLayer { alpha = overlayAlpha }) {
      if (shouldComposeThumbs && verticalMetrics.isVisible) {
        EditorScrollbarThumb(
          metrics = verticalMetrics,
          viewportState = viewportState,
          visibleArea = visibleArea,
          isUserScroll = wasLastScrollUser,
          inputEnabled = inputEnabled,
          isDragging = isVerticalThumbDragged,
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
            wasLastScrollUser = true
            if (!dragging && !isHorizontalThumbDragged) {
              hideSequenceRevision += 1
            }
          },
        )
      }
      if (shouldComposeThumbs && horizontalMetrics.isVisible) {
        EditorScrollbarThumb(
          horizontal = true,
          metrics = horizontalMetrics,
          viewportState = viewportState,
          visibleArea = visibleArea,
          isUserScroll = wasLastScrollUser,
          inputEnabled = inputEnabled,
          isDragging = isHorizontalThumbDragged,
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
            wasLastScrollUser = true
            if (!dragging && !isVerticalThumbDragged) {
              hideSequenceRevision += 1
            }
          },
        )
      }
    }

    if (
      indicatorText != null && (indicatorAlpha > 0f || indicatorVisible || isVerticalThumbDragged)
    ) {
      EditorScrollbarIndicator(
        metrics = verticalMetrics,
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
  isUserScroll: Boolean,
): Float =
  if (visible || dragging) {
    if (isUserScroll) 1f else 0.65f
  } else {
    0f
  }

internal fun resolveEditorScrollbarThumbThickness(dragging: Boolean): Float =
  if (dragging) ScrollbarActiveThumbWidth else ScrollbarThumbWidth

internal fun resolveEditorScrollbarThumbAlpha(dragging: Boolean, isUserScroll: Boolean): Float =
  if (isUserScroll) {
    if (dragging) 0.8f else 0.5f
  } else {
    if (dragging) 0.45f else 0.22f
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
private fun BoxScope.EditorScrollbarIndicator(
  metrics: EditorViewportScrollbarMetrics,
  visibleArea: EditorVisibleArea,
  text: String,
  alpha: Float,
) {
  val density = LocalDensity.current
  val top =
    visibleArea.topOcclusion +
      ScrollbarTrackPadding +
      metrics.thumbOffset +
      metrics.thumbSize / 2f - ScrollbarIndicatorHeight / 2f
  val right = ScrollbarTrackPadding + ScrollbarActiveThumbWidth + ScrollbarIndicatorGap

  Box(
    modifier =
      Modifier.align(Alignment.TopEnd)
        .graphicsLayer {
          this.alpha = alpha
          translationX = with(density) { -right.dp.toPx() }
          translationY = with(density) { top.dp.toPx() }
        }
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

@Composable
private fun EditorScrollbarThumb(
  horizontal: Boolean = false,
  metrics: EditorViewportScrollbarMetrics,
  viewportState: EditorViewportState,
  visibleArea: EditorVisibleArea,
  isUserScroll: Boolean,
  inputEnabled: Boolean,
  isDragging: Boolean,
  onDragChanged: (Boolean) -> Unit,
) {
  val density = LocalDensity.current
  val viewportSize = viewportState.viewportSize
  val contentSize = viewportState.contentSize
  val hitThickness = (ScrollbarTrackWidth + ScrollbarLongPressHitExpansion).dp
  val trackWidth = ScrollbarTrackWidth.dp
  val trackPadding = ScrollbarTrackPadding.dp
  val animatedThumbThickness by
    animateDpAsState(
      targetValue = Dp(resolveEditorScrollbarThumbThickness(isDragging)),
      animationSpec = tween(durationMillis = ScrollbarThumbAnimationMs, easing = EaseInOutBack),
      label = "editor-scrollbar-thumb-thickness",
    )
  val animatedThumbAlpha by
    animateFloatAsState(
      targetValue =
        resolveEditorScrollbarThumbAlpha(dragging = isDragging, isUserScroll = isUserScroll),
      animationSpec = tween(durationMillis = ScrollbarThumbAnimationMs, easing = EaseInOutBack),
      label = "editor-scrollbar-thumb-alpha",
    )

  val translation =
    if (!horizontal) {
      Offset(
        x =
          (viewportSize.width - ScrollbarTrackWidth - ScrollbarLongPressHitExpansion).coerceAtLeast(
            0f
          ),
        y =
          (visibleArea.topOcclusion + ScrollbarTrackPadding + metrics.thumbOffset).coerceAtLeast(0f),
      )
    } else {
      Offset(
        x = (ScrollbarTrackPadding + metrics.thumbOffset).coerceAtLeast(0f),
        y =
          (viewportSize.height -
              visibleArea.bottomOcclusion -
              ScrollbarTrackWidth -
              ScrollbarLongPressHitExpansion)
            .coerceAtLeast(0f),
      )
    }

  val size =
    if (!horizontal) {
      Pair(hitThickness, Dp(metrics.thumbSize))
    } else {
      Pair(Dp(metrics.thumbSize), hitThickness)
    }

  Box(
    modifier =
      Modifier.graphicsLayer {
          translationX = with(density) { translation.x.dp.toPx() }
          translationY = with(density) { translation.y.dp.toPx() }
        }
        .size(width = size.first, height = size.second)
        .then(
          if (inputEnabled) {
            Modifier.pointerInput(
              horizontal,
              metrics.trackLength,
              metrics.thumbSize,
              viewportSize,
              contentSize,
              viewportState,
            ) {
              var dragStartScroll = 0f
              var accumulatedDrag = 0f

              detectDragGestures(
                onDragStart = {
                  dragStartScroll =
                    if (!horizontal) {
                      viewportState.scrollOffset.y
                    } else {
                      viewportState.scrollOffset.x
                    }
                  accumulatedDrag = 0f
                  onDragChanged(true)
                },
                onDragEnd = { onDragChanged(false) },
                onDragCancel = { onDragChanged(false) },
              ) { change, dragAmount ->
                change.consume()
                accumulatedDrag +=
                  with(density) {
                    if (!horizontal) {
                      dragAmount.y.toDp().value
                    } else {
                      dragAmount.x.toDp().value
                    }
                  }

                val nextScrollPosition =
                  resolveEditorViewportScrollbarScrollPositionFromDrag(
                    startScrollPosition = dragStartScroll,
                    dragDelta = accumulatedDrag,
                    trackLength = metrics.trackLength,
                    thumbSize = metrics.thumbSize,
                    viewportLength = if (!horizontal) viewportSize.height else viewportSize.width,
                    contentLength = if (!horizontal) contentSize.height else contentSize.width,
                  )

                if (!horizontal) {
                  viewportState.scrollTo(
                    offset = Offset(x = viewportState.scrollOffset.x, y = nextScrollPosition),
                    isUserScroll = true,
                  )
                } else {
                  viewportState.scrollTo(
                    offset = Offset(x = nextScrollPosition, y = viewportState.scrollOffset.y),
                    isUserScroll = true,
                  )
                }
              }
            }
          } else {
            Modifier
          }
        )
  ) {
    if (!horizontal) {
      Box(
        modifier =
          Modifier.align(Alignment.CenterEnd)
            .size(width = trackWidth, height = size.second)
            .padding(end = trackPadding),
        contentAlignment = Alignment.CenterEnd,
      ) {
        EditorScrollbarThumbBody(
          modifier = Modifier.size(width = animatedThumbThickness, height = size.second),
          alpha = animatedThumbAlpha,
        )
      }
    } else {
      Box(
        modifier =
          Modifier.align(Alignment.BottomCenter)
            .size(width = size.first, height = trackWidth)
            .padding(bottom = trackPadding),
        contentAlignment = Alignment.BottomCenter,
      ) {
        EditorScrollbarThumbBody(
          modifier = Modifier.size(width = size.first, height = animatedThumbThickness),
          alpha = animatedThumbAlpha,
        )
      }
    }
  }
}

@Composable
private fun EditorScrollbarThumbBody(modifier: Modifier, alpha: Float) {
  Box(
    modifier =
      modifier.clip(ScrollbarShape).background(AppTheme.colors.surfaceInverse.copy(alpha = alpha))
  )
}
