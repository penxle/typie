package co.typie.screen.stats

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.icons.Lucide
import co.typie.ui.component.TapGestureMovementTolerancePx
import co.typie.ui.component.Text
import co.typie.ui.component.TooltipGestureAction
import co.typie.ui.component.TooltipGesturePhase
import co.typie.ui.component.resolveTooltipGestureAction
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.datetime.number
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.roundToInt

private val ChartHeight = 100.dp
private val ChartAxisHeight = 24.dp
private val ChartArrowWidth = 24.dp

@Composable
fun StatsActivityChart(
  characterCountChanges: List<StatsCharacterCountChange>,
  horizontalPadding: Int = 16,
  onVerticalScrollDelta: (Float) -> Unit = {},
) {
  val daysData = remember(characterCountChanges) { generateActivityChartDays(characterCountChanges) }
  var showAdditions by remember { mutableStateOf(true) }
  var showDeletions by remember { mutableStateOf(true) }
  var selectedIndex by remember { mutableStateOf<Int?>(null) }
  var zoom by remember { mutableFloatStateOf(1f) }
  var visualScrollOffset by remember { mutableFloatStateOf(0f) }
  var tooltipAutoHideGeneration by remember { mutableIntStateOf(0) }
  var tooltipAutoHideArmed by remember { mutableStateOf(false) }
  var manualScrollActive by remember { mutableStateOf(false) }
  var isPinching by remember { mutableStateOf(false) }
  val scrollState = rememberScrollState()
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val haptic = LocalHapticFeedback.current
  val textMeasurer = rememberTextMeasurer()
  val colors = AppTheme.colors

  LaunchedEffect(daysData.size) {
    selectedIndex = null
    tooltipAutoHideArmed = false
    zoom = 1f
    visualScrollOffset = 0f
    scrollState.scrollTo(0)
  }

  Column(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Text(
      "지난 3개월간의 기록",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textSubtle,
      modifier = Modifier.padding(horizontal = horizontalPadding.dp),
    )

    Spacer(Modifier.height(24.dp))

    BoxWithConstraints(
      modifier = Modifier
        .fillMaxWidth()
        .height(ChartHeight + ChartAxisHeight),
    ) {
      val horizontalPaddingPx = with(density) { horizontalPadding.dp.toPx() }
      val viewportWidthDp = maxWidth - (horizontalPadding * 2).dp
      val viewportWidthPx = with(density) { viewportWidthDp.coerceAtLeast(1.dp).toPx() }
      val contentWidthPx = viewportWidthPx * zoom
      val contentWidthDp = with(density) { contentWidthPx.toDp() }
      val chartHeightPx = with(density) { ChartHeight.toPx() }
      val currentScrollOffset = if (isPinching) visualScrollOffset else scrollState.value.toFloat()
      val visualScrollDelta = scrollState.value.toFloat() - currentScrollOffset
      val barWidthPx = if (daysData.isEmpty()) 0f else contentWidthPx / daysData.size
      val maxScrollOffset = max(contentWidthPx - viewportWidthPx, 0f)
      val hasHorizontalOverflow = maxScrollOffset > 0.5f
      val visibleRange = remember(daysData.size, barWidthPx, viewportWidthPx, currentScrollOffset) {
        calculateChartVisibleRange(
          length = daysData.size,
          barWidthPx = barWidthPx,
          viewportWidthPx = viewportWidthPx,
          scrollOffset = currentScrollOffset,
        )
      }
      val maxValue = remember(daysData, showAdditions, showDeletions, visibleRange) {
        chartMaxValue(
          daysData = daysData,
          showAdditions = showAdditions,
          showDeletions = showDeletions,
          startIndex = visibleRange.start,
          endIndex = visibleRange.end,
        )
      }
      val animatedMaxValue by animateFloatAsState(
        targetValue = maxValue.toFloat(),
        animationSpec = tween(durationMillis = 180),
      )
      val canScrollLeft by remember(currentScrollOffset, hasHorizontalOverflow) {
        derivedStateOf { hasHorizontalOverflow && currentScrollOffset > 0.5f }
      }
      val canScrollRight by remember(currentScrollOffset, hasHorizontalOverflow, maxScrollOffset) {
        derivedStateOf { hasHorizontalOverflow && currentScrollOffset < maxScrollOffset - 0.5f }
      }
      val xAxisLabels = remember(daysData, zoom) { generateXAxisLabels(daysData, zoom) }
      val labelStyle = AppTheme.typography.caption
      val positionedLabels = remember(xAxisLabels, barWidthPx, contentWidthPx, labelStyle) {
        positionXAxisLabels(
          labels = xAxisLabels,
          barWidthPx = barWidthPx,
          chartWidthPx = contentWidthPx,
          textMeasurer = textMeasurer,
          textStyle = labelStyle,
        )
      }
      val interactionMetrics by rememberUpdatedState(
        ChartInteractionMetrics(
          barWidthPx = barWidthPx,
          itemCount = daysData.size,
          hasHorizontalOverflow = hasHorizontalOverflow,
        ),
      )

      fun barIndexAt(x: Float): Int? {
        val metrics = interactionMetrics
        return barIndexAtContentPosition(
          localContentX = x,
          barWidthPx = metrics.barWidthPx,
          itemCount = metrics.itemCount,
        )
      }

      fun clearSelection() {
        tooltipAutoHideArmed = false
        selectedIndex = null
      }

      fun showSelection(localPosition: Offset, withHaptic: Boolean) {
        val nextIndex = barIndexAt(localPosition.x) ?: return
        val isSelectionChanged = selectedIndex != nextIndex
        tooltipAutoHideArmed = false

        if (withHaptic && isSelectionChanged) {
          haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
        }

        selectedIndex = nextIndex
      }

      fun hideAfterDelay() {
        tooltipAutoHideGeneration += 1
        tooltipAutoHideArmed = true
      }

      fun applyManualScroll(deltaX: Float) {
        val requested = -deltaX
        val minDelta = -scrollState.value.toFloat()
        val maxDelta = (scrollState.maxValue - scrollState.value).toFloat()
        val clampedDelta = requested.coerceIn(minDelta, maxDelta)
        if (clampedDelta != 0f) {
          scrollState.dispatchRawDelta(clampedDelta)
          visualScrollOffset = scrollState.value.toFloat()
        }
      }

      LaunchedEffect(scrollState, isPinching) {
        snapshotFlow { scrollState.value }
          .collectLatest { value ->
            if (!isPinching) {
              visualScrollOffset = value.toFloat()
            }
          }
      }

      LaunchedEffect(scrollState, isPinching) {
        snapshotFlow { scrollState.isScrollInProgress }
          .collectLatest { isScrolling ->
            if (isScrolling && !isPinching) {
              clearSelection()
            }
          }
      }

      LaunchedEffect(selectedIndex, tooltipAutoHideGeneration, tooltipAutoHideArmed) {
        if (selectedIndex == null || !tooltipAutoHideArmed) {
          return@LaunchedEffect
        }

        delay(1_000)
        if (tooltipAutoHideArmed) {
          clearSelection()
        }
      }

      LaunchedEffect(isPinching, visualScrollOffset, maxScrollOffset) {
        if (isPinching) {
          return@LaunchedEffect
        }

        val clampedOffset = visualScrollOffset.coerceIn(0f, maxScrollOffset)
        if (clampedOffset != visualScrollOffset) {
          visualScrollOffset = clampedOffset
        }

        val targetOffset = clampedOffset.roundToInt()
        if (scrollState.value != targetOffset) {
          scrollState.scrollTo(targetOffset)
        }
      }

      val chartPinchModifier = Modifier.pointerInput(daysData.size, viewportWidthPx, horizontalPadding) {
        awaitEachGesture {
          awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Initial)
          var pinchStarted = false
          var pinchStartZoom = 1f
          var pinchStartOffset = 0f
          var pinchStartFocalX = 0f
          var pinchStartDistance = 1f

          while (true) {
            val event = awaitPointerEvent(pass = PointerEventPass.Initial)
            val pressedChanges = event.changes.filter { it.pressed }

            if (pressedChanges.isEmpty()) {
              if (pinchStarted) {
                isPinching = false
              }
              break
            }

            if (pressedChanges.size < 2) {
              if (pinchStarted) {
                isPinching = false
                break
              }
              continue
            }

            val first = pressedChanges[0]
            val second = pressedChanges[1]
            if (!pinchStarted) {
              pinchStarted = true
              val gestureStartScrollOffset = if (isPinching) visualScrollOffset else scrollState.value.toFloat()
              pinchStartZoom = zoom
              pinchStartOffset = gestureStartScrollOffset
              pinchStartFocalX = viewportFocalXFromContentPosition(
                localContentX = (first.position.x + second.position.x) / 2f,
                scrollOffset = gestureStartScrollOffset,
                viewportWidthPx = viewportWidthPx,
              )
              pinchStartDistance = (first.position - second.position).getDistance().coerceAtLeast(1f)
              isPinching = true
              manualScrollActive = false
              clearSelection()
            }

            val currentDistance = (first.position - second.position).getDistance().coerceAtLeast(1f)
            val nextZoomState = calculateChartPinchZoomState(
              pinchStartZoom = pinchStartZoom,
              pinchScale = currentDistance / pinchStartDistance,
              pinchStartOffset = pinchStartOffset,
              focalX = pinchStartFocalX,
              viewportWidth = viewportWidthPx,
            )

            zoom = nextZoomState.zoom
            visualScrollOffset = nextZoomState.scrollOffset
            event.changes.forEach { change ->
              if (change.pressed) {
                change.consume()
              }
            }
          }
        }
      }

      val chartGestureModifier = Modifier.pointerInput(daysData.size, viewportWidthPx, horizontalPadding) {
        awaitEachGesture {
          val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Final)
          var activePointerId = down.id
          var currentChange = down
          var startPosition = down.position
          var velocityTracker = VelocityTracker().apply {
            addPosition(down.uptimeMillis, down.position)
          }
          var isTooltipGesture = selectedIndex != null
          var isScrubGesture = false
          var isVerticalScrollGesture = false
          var isScrollGesture = false

          if (isTooltipGesture) {
            showSelection(down.position, withHaptic = false)
          } else {
            val preGestureResult = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) {
              while (true) {
                val event = awaitPointerEvent(pass = PointerEventPass.Final)
                if (event.changes.count { it.pressed } > 1) {
                  return@withTimeoutOrNull ChartPreGestureResult.Cancel
                }

                val change = event.changes.firstOrNull { it.id == activePointerId }
                  ?: event.changes.firstOrNull { it.pressed }?.also { activePointerId = it.id }
                  ?: return@withTimeoutOrNull ChartPreGestureResult.Cancel

                currentChange = change

                if (change.isConsumed) {
                  return@withTimeoutOrNull ChartPreGestureResult.Cancel
                }

                if (!change.pressed) {
                  return@withTimeoutOrNull ChartPreGestureResult.Tap(change.position)
                }

                if ((change.position - startPosition).getDistance() > TapGestureMovementTolerancePx) {
                  return@withTimeoutOrNull ChartPreGestureResult.Cancel
                }
              }
            }

            when (preGestureResult) {
              is ChartPreGestureResult.Tap -> {
                showSelection(preGestureResult.position, withHaptic = false)
                hideAfterDelay()
                return@awaitEachGesture
              }

              ChartPreGestureResult.Cancel -> {
                return@awaitEachGesture
              }

              null -> {
                isTooltipGesture = true
                tooltipAutoHideArmed = false
                startPosition = currentChange.position
                velocityTracker = VelocityTracker().apply {
                  addPosition(currentChange.uptimeMillis, currentChange.position)
                }
                showSelection(currentChange.position, withHaptic = true)
              }
            }
          }

          try {
            while (true) {
              val event = awaitPointerEvent(
                pass = if (isTooltipGesture || isScrubGesture || isVerticalScrollGesture || isScrollGesture) {
                  PointerEventPass.Main
                } else {
                  PointerEventPass.Final
                },
              )

              if (event.changes.count { it.pressed } > 1) {
                clearSelection()
                break
              }

              val change = event.changes.firstOrNull { it.id == activePointerId }
                ?: event.changes.firstOrNull { it.pressed }?.also { activePointerId = it.id }
                ?: break

              if (!change.pressed) {
                if (!isScrollGesture && selectedIndex != null) {
                  hideAfterDelay()
                }
                break
              }

              if (!isScrubGesture && !isScrollGesture && change.isConsumed) {
                clearSelection()
                break
              }

              val deltaX = change.position.x - change.previousPosition.x
              val deltaY = change.position.y - change.previousPosition.y
              velocityTracker.addPosition(change.uptimeMillis, change.position)

              if (isVerticalScrollGesture) {
                change.consume()
                onVerticalScrollDelta(-deltaY)
                continue
              }

              if (!isScrollGesture && isTooltipGesture) {
                val velocity = velocityTracker.calculateVelocity()
                val action = resolveTooltipGestureAction(
                  phase = if (isScrubGesture) TooltipGesturePhase.Scrub else TooltipGesturePhase.Tooltip,
                  velocityX = velocity.x,
                  velocityY = velocity.y,
                ).let { resolved ->
                  if (resolved == TooltipGestureAction.BeginHorizontalScroll && !interactionMetrics.hasHorizontalOverflow) {
                    if (isScrubGesture) TooltipGestureAction.ContinueScrub else TooltipGestureAction.BeginScrub
                  } else {
                    resolved
                  }
                }

                when (action) {
                  TooltipGestureAction.BeginHorizontalScroll -> {
                    clearSelection()
                    isScrubGesture = false
                    isScrollGesture = true
                    manualScrollActive = true
                    change.consume()
                    applyManualScroll(deltaX)
                  }

                  TooltipGestureAction.BeginVerticalScroll -> {
                    clearSelection()
                    isScrubGesture = false
                    isTooltipGesture = false
                    isVerticalScrollGesture = true
                    change.consume()
                    onVerticalScrollDelta(-deltaY)
                  }

                  TooltipGestureAction.BeginScrub -> {
                    isScrubGesture = true
                    change.consume()
                    showSelection(change.position, withHaptic = true)
                  }

                  TooltipGestureAction.ContinueScrub -> {
                    change.consume()
                    showSelection(change.position, withHaptic = true)
                  }
                }
                continue
              }

              if (isScrollGesture) {
                change.consume()
                applyManualScroll(deltaX)
              }
            }
          } finally {
            manualScrollActive = false
          }
        }
      }

      Box(
        modifier = Modifier.fillMaxWidth(),
      ) {
        Box(
          modifier = Modifier
            .fillMaxWidth()
            .height(ChartHeight + ChartAxisHeight)
            .horizontalScroll(
              scrollState,
              enabled = hasHorizontalOverflow && selectedIndex == null && !manualScrollActive && !isPinching,
            )
            .padding(horizontal = horizontalPadding.dp),
        ) {
          Column(
            modifier = Modifier
              .width(contentWidthDp)
              .offset { IntOffset(visualScrollDelta.roundToInt(), 0) },
          ) {
            Box(
              modifier = Modifier
                .fillMaxWidth()
                .height(ChartHeight)
                .then(chartPinchModifier)
                .then(chartGestureModifier),
            ) {
              Canvas(
                modifier = Modifier.fillMaxSize(),
              ) {
                val gridLineColor = colors.borderSubtle.copy(alpha = 0.7f)
                val additionColor = colors.accentSuccess
                val deletionColor = colors.surfaceDark
                val zeroBarColor = colors.borderStrong
                val selectionColor = colors.surfaceDark

                for (lineIndex in 1..5) {
                  val y = chartHeightPx - (lineIndex * (chartHeightPx / 5f))
                  drawLine(
                    color = gridLineColor,
                    start = Offset(0f, y),
                    end = Offset(size.width, y),
                    strokeWidth = 1f,
                  )
                }

                daysData.forEachIndexed { index, day ->
                  val left = index * barWidthPx + 1f
                  val width = max(barWidthPx - 2f, 0f)
                  val additions = if (showAdditions) day.additions else 0
                  val deletions = if (showDeletions) day.deletions else 0
                  val total = additions + deletions
                  val heights = calculateChartBarHeights(
                    additions = additions,
                    deletions = deletions,
                    maxValue = animatedMaxValue,
                    chartHeightPx = chartHeightPx,
                  )
                  val additionsHeight = heights.additionsHeightPx
                  val deletionsHeight = heights.deletionsHeightPx

                  if (deletions > 0) {
                    val height = max(deletionsHeight, 1f)
                    val bottom = if (additionsHeight > 0f) additionsHeight + 1f else 0f
                    drawRoundRect(
                      color = deletionColor,
                      topLeft = Offset(left, chartHeightPx - bottom - height),
                      size = androidx.compose.ui.geometry.Size(width, height),
                      cornerRadius = CornerRadius(1f, 1f),
                    )
                  }

                  if (additions > 0) {
                    val height = max(additionsHeight, 1f)
                    drawRoundRect(
                      color = additionColor,
                      topLeft = Offset(left, chartHeightPx - height),
                      size = androidx.compose.ui.geometry.Size(width, height),
                      cornerRadius = CornerRadius(1f, 1f),
                    )
                  }

                  if (total == 0) {
                    drawRoundRect(
                      color = zeroBarColor,
                      topLeft = Offset(left, chartHeightPx - 1f),
                      size = androidx.compose.ui.geometry.Size(width, 1f),
                      cornerRadius = CornerRadius(1f, 1f),
                    )
                  }

                  if (selectedIndex == index) {
                    drawRoundRect(
                      color = selectionColor,
                      topLeft = Offset(index * barWidthPx, 0f),
                      size = androidx.compose.ui.geometry.Size(barWidthPx, chartHeightPx),
                      cornerRadius = CornerRadius(2f, 2f),
                      style = Stroke(1f),
                    )
                  }
                }
              }
            }

            Box(
              modifier = Modifier
                .fillMaxWidth()
                .height(ChartAxisHeight),
            ) {
              positionedLabels.forEach { label ->
                Text(
                  label.text,
                  style = AppTheme.typography.caption,
                  color = colors.textFaint,
                  modifier = Modifier.offset { IntOffset(label.left.roundToInt(), 0) },
                )
              }
            }
          }
        }

        val selectedDay = selectedIndex?.let { daysData.getOrNull(it) }
        if (selectedDay != null && selectedIndex != null) {
          val anchorCenterX = horizontalPaddingPx +
            (selectedIndex!! * barWidthPx) +
            (barWidthPx / 2f) -
            currentScrollOffset
          Layout(
            modifier = Modifier
              .align(Alignment.TopStart)
              .fillMaxSize(),
            content = {
              Box(
                modifier = Modifier
                  .clip(androidx.compose.foundation.shape.RoundedCornerShape(6.dp))
                  .background(AppTheme.colors.surfaceDark)
                  .padding(horizontal = 12.dp, vertical = 8.dp),
              ) {
                Column(
                  verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                  Text(
                    formatFullDate(selectedDay.date),
                    style = AppTheme.typography.caption,
                    color = colors.textBright,
                  )
                  if (selectedDay.additions > 0) {
                    Text(
                      "입력: ${selectedDay.additions.formatGrouped()}자",
                      style = AppTheme.typography.micro,
                      color = colors.textBright,
                    )
                  }
                  if (selectedDay.deletions > 0) {
                    Text(
                      "지움: ${selectedDay.deletions.formatGrouped()}자",
                      style = AppTheme.typography.micro,
                      color = colors.textBright,
                    )
                  }
                  if (selectedDay.total == 0) {
                    Text(
                      "기록이 없어요",
                      style = AppTheme.typography.micro,
                      color = colors.textBright,
                    )
                  }
                }
              }
            },
          ) { measurables, constraints ->
            val placeable = measurables.first().measure(constraints.copy(minWidth = 0, minHeight = 0))
            val tooltipOffset = calculateChartTooltipOffset(
              anchorCenterXInChartPx = anchorCenterX,
              tooltipWidthPx = placeable.width.toFloat(),
              tooltipHeightPx = placeable.height.toFloat(),
            )

            layout(constraints.maxWidth, constraints.maxHeight) {
              placeable.placeRelative(
                tooltipOffset.x.roundToInt(),
                tooltipOffset.y.roundToInt(),
              )
            }
          }
        }

        if (canScrollLeft) {
          Box(
            modifier = Modifier
              .align(Alignment.CenterStart)
              .width(ChartArrowWidth)
              .height(ChartHeight)
              .background(
                Brush.horizontalGradient(
                  colors = listOf(
                    colors.surfaceSubtle.copy(alpha = 0.88f),
                    colors.surfaceSubtle.copy(alpha = 0f),
                  ),
                ),
              )
              .clickable {
                scope.launch {
                  scrollState.animateScrollBy(-(viewportWidthPx * 0.75f))
                }
              },
            contentAlignment = Alignment.CenterStart,
          ) {
            Icon(
              icon = Lucide.ChevronLeft,
              tint = colors.textSubtle,
            )
          }
        }

        if (canScrollRight) {
          Box(
            modifier = Modifier
              .align(Alignment.CenterEnd)
              .width(ChartArrowWidth)
              .height(ChartHeight)
              .background(
                Brush.horizontalGradient(
                  colors = listOf(
                    colors.surfaceSubtle.copy(alpha = 0f),
                    colors.surfaceSubtle.copy(alpha = 0.88f),
                  ),
                ),
              )
              .clickable {
                scope.launch {
                  scrollState.animateScrollBy(viewportWidthPx * 0.75f)
                }
              },
            contentAlignment = Alignment.CenterEnd,
          ) {
            Icon(
              icon = Lucide.ChevronRight,
              tint = colors.textSubtle,
            )
          }
        }
      }
    }

    Spacer(Modifier.height(8.dp))

    Row(
      modifier = Modifier
        .fillMaxWidth()
        .padding(horizontal = horizontalPadding.dp),
      horizontalArrangement = Arrangement.End,
    ) {
      ChartLegendToggle(
        label = "입력한 글자",
        color = AppTheme.colors.accentSuccess,
        selected = showAdditions,
        onClick = { showAdditions = !showAdditions },
      )
      Spacer(Modifier.width(16.dp))
      ChartLegendToggle(
        label = "지운 글자",
        color = AppTheme.colors.surfaceDark,
        selected = showDeletions,
        onClick = { showDeletions = !showDeletions },
      )
    }
  }
}

@Composable
private fun ChartLegendToggle(
  label: String,
  color: Color,
  selected: Boolean,
  onClick: () -> Unit,
) {
  Row(
    modifier = Modifier.clickable(onClick),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Box(
      modifier = Modifier
        .width(12.dp)
        .height(12.dp)
        .clip(androidx.compose.foundation.shape.RoundedCornerShape(2.dp))
        .background(if (selected) color else color.copy(alpha = 0.3f)),
    )
    Spacer(Modifier.width(6.dp))
    Text(
      label,
      style = AppTheme.typography.caption,
      color = if (selected) AppTheme.colors.textSubtle else AppTheme.colors.textFaint,
    )
  }
}

internal data class ChartPinchZoomState(
  val zoom: Float,
  val scrollOffset: Float,
)

internal data class ChartVisibleRange(
  val start: Int,
  val end: Int,
)

private data class ChartInteractionMetrics(
  val barWidthPx: Float,
  val itemCount: Int,
  val hasHorizontalOverflow: Boolean,
)

internal data class ChartXAxisLabel(
  val index: Int,
  val text: String,
)

private data class PositionedChartXAxisLabel(
  val text: String,
  val left: Float,
)

internal data class ChartBarHeights(
  val additionsHeightPx: Float,
  val deletionsHeightPx: Float,
)

private sealed interface ChartPreGestureResult {
  data class Tap(val position: Offset) : ChartPreGestureResult
  object Cancel : ChartPreGestureResult
}

internal fun calculateChartTooltipOffset(
  anchorCenterXInChartPx: Float,
  tooltipWidthPx: Float,
  tooltipHeightPx: Float,
  gapPx: Float = 4f,
): Offset {
  return Offset(
    x = anchorCenterXInChartPx - (tooltipWidthPx / 2f),
    y = -tooltipHeightPx - gapPx,
  )
}

internal fun barIndexAtContentPosition(
  localContentX: Float,
  barWidthPx: Float,
  itemCount: Int,
): Int? {
  if (itemCount <= 0 || barWidthPx <= 0f || localContentX < 0f) {
    return null
  }

  val index = (localContentX / barWidthPx).toInt()
  return index.takeIf { it in 0 until itemCount }
}

internal fun calculateChartBarHeights(
  additions: Int,
  deletions: Int,
  maxValue: Float,
  chartHeightPx: Float,
): ChartBarHeights {
  val safeMaxValue = max(maxValue, 1f)

  return ChartBarHeights(
    additionsHeightPx = if (additions > 0) (additions / safeMaxValue) * chartHeightPx else 0f,
    deletionsHeightPx = if (deletions > 0) (deletions / safeMaxValue) * chartHeightPx else 0f,
  )
}

internal fun viewportFocalXFromContentPosition(
  localContentX: Float,
  scrollOffset: Float,
  viewportWidthPx: Float,
): Float {
  return (localContentX - scrollOffset).coerceIn(0f, viewportWidthPx)
}

internal fun calculateChartPinchZoomState(
  pinchStartZoom: Float,
  pinchScale: Float,
  pinchStartOffset: Float,
  focalX: Float,
  viewportWidth: Float,
  minZoom: Float = 1f,
  maxZoom: Float = 4f,
): ChartPinchZoomState {
  val nextZoom = (pinchStartZoom * pinchScale).coerceIn(minZoom, maxZoom)
  val clampedFocalX = focalX.coerceIn(0f, viewportWidth)
  val contentX = pinchStartOffset + clampedFocalX
  val targetOffset = if (pinchStartZoom <= 0f) {
    0f
  } else {
    contentX * (nextZoom / pinchStartZoom) - clampedFocalX
  }
  val maxOffset = max(viewportWidth * nextZoom - viewportWidth, 0f)

  return ChartPinchZoomState(
    zoom = nextZoom,
    scrollOffset = targetOffset.coerceIn(0f, maxOffset),
  )
}

internal fun calculateChartVisibleRange(
  length: Int,
  barWidthPx: Float,
  viewportWidthPx: Float,
  scrollOffset: Float,
): ChartVisibleRange {
  if (length == 0 || barWidthPx <= 0f) {
    return ChartVisibleRange(start = 0, end = 0)
  }

  val start = (scrollOffset / barWidthPx).toInt().coerceIn(0, length - 1)
  val end = kotlin.math.ceil((scrollOffset + viewportWidthPx) / barWidthPx)
    .toInt()
    .coerceIn(start + 1, length)

  return ChartVisibleRange(start = start, end = end)
}

private fun chartMaxValue(
  daysData: List<StatsActivityDay>,
  showAdditions: Boolean,
  showDeletions: Boolean,
  startIndex: Int,
  endIndex: Int,
): Int {
  if (daysData.isEmpty() || endIndex <= startIndex) {
    return 1_000
  }

  val maxValue = daysData.subList(startIndex, endIndex).maxOfOrNull { day ->
    (if (showAdditions) day.additions else 0) + (if (showDeletions) day.deletions else 0)
  } ?: 0
  return max(maxValue, 1_000)
}

internal fun generateXAxisLabels(daysData: List<StatsActivityDay>, zoom: Float): List<ChartXAxisLabel> {
  if (daysData.isEmpty()) {
    return emptyList()
  }

  val labels = mutableListOf<ChartXAxisLabel>()
  val minGap = when {
    zoom >= 3f -> 2
    zoom >= 2.2f -> 3
    zoom >= 1.6f -> 4
    else -> 5
  }
  val showWeekly = zoom >= 1.6f
  val showDense = zoom >= 2.6f
  var lastShownIndex = -999

  daysData.forEachIndexed { index, day ->
    val isFirst = index == 0
    val isLast = index == daysData.lastIndex
    val isFirstOfMonth = day.date.day == 1
    val isIntervalLabel = if (showDense) {
      index % 3 == 0
    } else {
      showWeekly && index % 7 == 0
    }
    val shouldShowLabel = isFirst || isLast || isFirstOfMonth || isIntervalLabel

    if (!shouldShowLabel) {
      return@forEachIndexed
    }

    if (!isFirst && !isLast && index - lastShownIndex < minGap) {
      return@forEachIndexed
    }

    labels += ChartXAxisLabel(index = index, text = "${day.date.month.number}/${day.date.day}")
    lastShownIndex = index
  }

  val lastIndex = daysData.lastIndex
  if (labels.isNotEmpty() && labels.last().index != lastIndex) {
    if (lastIndex - labels.last().index < minGap && labels.size > 1) {
      labels.removeLast()
    }

    val day = daysData[lastIndex]
    labels += ChartXAxisLabel(index = lastIndex, text = "${day.date.month.number}/${day.date.day}")
  }

  return labels
}

private fun positionXAxisLabels(
  labels: List<ChartXAxisLabel>,
  barWidthPx: Float,
  chartWidthPx: Float,
  textMeasurer: androidx.compose.ui.text.TextMeasurer,
  textStyle: androidx.compose.ui.text.TextStyle,
): List<PositionedChartXAxisLabel> {
  if (labels.isEmpty() || barWidthPx <= 0f || chartWidthPx <= 0f) {
    return emptyList()
  }

  val positioned = mutableListOf<PositionedChartXAxisLabel>()
  var lastRight = Float.NEGATIVE_INFINITY

  labels.forEach { label ->
    val textWidth = textMeasurer.measure(AnnotatedString(label.text), style = textStyle).size.width.toFloat()
    var left = (label.index * barWidthPx) - (textWidth / 2f)
    left = left.coerceIn(0f, (chartWidthPx - textWidth).coerceAtLeast(0f))
    val right = left + textWidth

    if (left < lastRight + 8f) {
      return@forEach
    }

    positioned += PositionedChartXAxisLabel(
      text = label.text,
      left = left,
    )
    lastRight = right
  }

  return positioned
}
