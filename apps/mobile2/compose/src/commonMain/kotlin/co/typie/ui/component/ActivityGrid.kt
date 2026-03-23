package co.typie.ui.component

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.animateScrollBy
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.datetime.toLocalDate
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.icons.Lucide
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import kotlinx.datetime.LocalDate
import kotlinx.datetime.number
import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.max
import kotlin.math.roundToInt
import kotlin.time.Clock

data class ActivityGridChange(
  val date: LocalDate,
  val additions: Int,
)

@Composable
fun ActivityGrid(
  changes: List<ActivityGridChange>,
  modifier: Modifier = Modifier,
  onVerticalScrollDelta: (Float) -> Unit = {},
) {
  val endDate = remember { Clock.System.now().toLocalDate() }
  val startDate = remember(endDate) { LocalDate.fromEpochDays(endDate.toEpochDays() - 364) }
  val activities = remember(changes, startDate, endDate) {
    generateActivities(changes = changes, startDate = startDate, endDate = endDate)
  }
  val weeks = remember(activities) { generateWeeks(activities) }
  val monthSpans = remember(activities) { generateMonthSpans(activities) }
  val totalWidth = remember(weeks) {
    if (weeks.isEmpty()) 0.dp
    else (CellSize * weeks.size) + (CellGap * max(weeks.size - 1, 0))
  }
  val scope = rememberCoroutineScope()
  val isDark = AppTheme.colors.isDark
  val levelColors = remember(isDark) { activityLevelColors(isDark = isDark) }
  val density = LocalDensity.current
  val haptic = LocalHapticFeedback.current
  val selectionBorderColor = AppTheme.colors.surfaceDark
  var selectedCell by remember { mutableStateOf<ActivityGridSelection?>(null) }
  var tooltipData by remember { mutableStateOf<ActivityGridSelection?>(null) }
  var tooltipVisible by remember { mutableStateOf(false) }
  var tooltipAutoHideGeneration by remember { mutableIntStateOf(0) }
  var tooltipAutoHideArmed by remember { mutableStateOf(false) }
  var manualScrollActive by remember { mutableStateOf(false) }

  BoxWithConstraints(modifier = modifier) {
    val viewportWidth = maxWidth - (HorizontalPadding * 2)
    val cellStridePx = with(density) { (CellSize + CellGap).toPx() }
    val initialScrollOffsetPx = remember(totalWidth, viewportWidth, density) {
      with(density) {
        (totalWidth.toPx() - viewportWidth.toPx()).coerceAtLeast(0f).roundToInt()
      }
    }
    val scrollState = rememberScrollState(initial = initialScrollOffsetPx)
    val canScrollLeft by remember(scrollState) {
      derivedStateOf { scrollState.value > ScrollEdgeVisibilityThresholdPx }
    }
    val canScrollRight by remember(scrollState) {
      derivedStateOf { scrollState.value < scrollState.maxValue - ScrollEdgeVisibilityThresholdPx }
    }
    val leftArrowAlpha by animateFloatAsState(if (canScrollLeft) 1f else 0f, tween(100))
    val rightArrowAlpha by animateFloatAsState(if (canScrollRight) 1f else 0f, tween(100))

    fun clearSelection() {
      tooltipAutoHideArmed = false
      selectedCell = null
      tooltipVisible = false
    }

    fun scrollByMonths(monthDelta: Int) {
      if (monthSpans.isEmpty()) return

      val currentWeekIndex = floor(scrollState.value / cellStridePx).toInt()
      var currentMonthIndex = 0

      for (index in monthSpans.indices) {
        val span = monthSpans[index]
        if (span.start <= currentWeekIndex && span.end >= currentWeekIndex) {
          currentMonthIndex = index
          break
        }
        if (span.start > currentWeekIndex) {
          currentMonthIndex = index - 1
          break
        }
      }

      val targetMonthIndex = (currentMonthIndex + monthDelta).coerceIn(0, monthSpans.lastIndex)
      val targetOffset = monthSpans[targetMonthIndex].start * cellStridePx
      scope.launch {
        scrollState.animateScrollBy(targetOffset - scrollState.value)
      }
    }

    fun scrollToMonth(span: ActivityGridMonthSpan) {
      val monthWidth =
        ((span.end - span.start + 1) * cellStridePx) - with(density) { CellGap.toPx() }
      val monthStart = span.start * cellStridePx
      val targetOffset =
        monthStart + (monthWidth / 2f) - with(density) { viewportWidth.toPx() } / 2f

      scope.launch {
        scrollState.animateScrollBy(
          targetOffset.coerceIn(
            0f,
            scrollState.maxValue.toFloat()
          ) - scrollState.value
        )
      }
    }

    fun selectionAt(localPosition: Offset): ActivityGridSelection? {
      val weekIndex = floor(localPosition.x / cellStridePx).toInt()
      val dayIndex = floor(localPosition.y / cellStridePx).toInt()
      val activity = weeks.getOrNull(weekIndex)?.getOrNull(dayIndex)

      if (activity == null || activity.level < 0) {
        return null
      }

      return ActivityGridSelection(
        activity = activity,
        weekIndex = weekIndex,
        dayIndex = dayIndex,
      )
    }

    fun showSelection(localPosition: Offset, withHaptic: Boolean) {
      val nextSelection = selectionAt(localPosition) ?: return
      val previousSelection = selectedCell
      val isCellChanged = previousSelection == null ||
        previousSelection.weekIndex != nextSelection.weekIndex ||
        previousSelection.dayIndex != nextSelection.dayIndex

      tooltipAutoHideArmed = false

      if (withHaptic && isCellChanged) {
        haptic.performHapticFeedback(HapticFeedbackType.TextHandleMove)
      }

      selectedCell = nextSelection
      tooltipData = nextSelection
      tooltipVisible = true
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
      }
    }

    LaunchedEffect(scrollState) {
      snapshotFlow { scrollState.isScrollInProgress }
        .collectLatest { isScrolling ->
          if (isScrolling) {
            clearSelection()
          }
        }
    }

    LaunchedEffect(selectedCell, tooltipAutoHideGeneration, tooltipAutoHideArmed) {
      if (selectedCell == null || !tooltipAutoHideArmed) {
        return@LaunchedEffect
      }

      delay(1_000)
      if (tooltipAutoHideArmed) {
        selectedCell = null
        tooltipVisible = false
        tooltipAutoHideArmed = false
      }
    }

    LaunchedEffect(tooltipVisible, tooltipData) {
      if (!tooltipVisible && tooltipData != null) {
        delay(140)
        if (!tooltipVisible) {
          tooltipData = null
        }
      }
    }

    val gridGestureModifier = Modifier.pointerInput(weeks, scrollState) {
      awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false, pass = PointerEventPass.Final)
        var activePointerId = down.id
        var currentChange = down
        var startPosition = down.position
        var velocityTracker = VelocityTracker().apply {
          addPosition(down.uptimeMillis, down.position)
        }
        var isTooltipGesture = tooltipVisible
        var isScrubGesture = false
        var isVerticalScrollGesture = false
        var isScrollGesture = false

        if (isTooltipGesture) {
          tooltipAutoHideArmed = false
          showSelection(down.position, withHaptic = false)
        } else {
          val preGestureResult = withTimeoutOrNull(viewConfiguration.longPressTimeoutMillis) {
            while (true) {
              val event = awaitPointerEvent(pass = PointerEventPass.Final)
              if (event.changes.count { it.pressed } > 1) {
                return@withTimeoutOrNull ActivityGridPreGestureResult.Cancel
              }

              val change = event.changes.firstOrNull { it.id == activePointerId }
                ?: event.changes.firstOrNull { it.pressed }?.also { activePointerId = it.id }
                ?: return@withTimeoutOrNull ActivityGridPreGestureResult.Cancel

              currentChange = change

              if (change.isConsumed) {
                return@withTimeoutOrNull ActivityGridPreGestureResult.Cancel
              }

              if (!change.pressed) {
                return@withTimeoutOrNull ActivityGridPreGestureResult.Tap(change.position)
              }

              if ((change.position - startPosition).getDistance() > TapGestureMovementTolerancePx) {
                return@withTimeoutOrNull ActivityGridPreGestureResult.Cancel
              }
            }
          }

          when (preGestureResult) {
            is ActivityGridPreGestureResult.Tap -> {
              showSelection(preGestureResult.position, withHaptic = false)
              hideAfterDelay()
              return@awaitEachGesture
            }

            ActivityGridPreGestureResult.Cancel -> {
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
              if (!isScrollGesture && selectedCell != null) {
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
            val velocity = velocityTracker.calculateVelocity()
            val velocityX = abs(velocity.x)
            val velocityY = abs(velocity.y)
            val isHorizontalVelocity = velocityX >= velocityY

            if (isVerticalScrollGesture) {
              change.consume()
              onVerticalScrollDelta(-deltaY)
              continue
            }

            if (!isScrollGesture && isTooltipGesture) {
              if (isScrubGesture) {
                if (isHorizontalVelocity && velocityX >= TooltipScrollVelocityThresholdPxPerSecond) {
                  clearSelection()
                  isScrubGesture = false
                  isScrollGesture = true
                  manualScrollActive = true
                  change.consume()
                  applyManualScroll(deltaX)
                  continue
                }

                if (!isHorizontalVelocity && velocityY >= TooltipScrollVelocityThresholdPxPerSecond) {
                  clearSelection()
                  isScrubGesture = false
                  isTooltipGesture = false
                  isVerticalScrollGesture = true
                  change.consume()
                  onVerticalScrollDelta(-deltaY)
                  continue
                }

                change.consume()
                showSelection(change.position, withHaptic = true)
                continue
              }

              if (isHorizontalVelocity && velocityX >= TooltipScrollVelocityThresholdPxPerSecond) {
                clearSelection()
                isScrollGesture = true
                manualScrollActive = true
                change.consume()
                applyManualScroll(deltaX)
                continue
              }

              if (!isHorizontalVelocity && velocityY >= TooltipScrollVelocityThresholdPxPerSecond) {
                clearSelection()
                isTooltipGesture = false
                isVerticalScrollGesture = true
                change.consume()
                onVerticalScrollDelta(-deltaY)
                continue
              }

              isScrubGesture = true
              change.consume()
              showSelection(change.position, withHaptic = true)
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
      modifier = Modifier
        .fillMaxWidth()
        .height(GridContainerHeight),
    ) {
      Box(
        modifier = Modifier
          .fillMaxSize(),
      ) {
        Box(
          modifier = Modifier
            .fillMaxSize()
            .horizontalScroll(scrollState, enabled = !tooltipVisible && !manualScrollActive)
            .padding(horizontal = HorizontalPadding)
            .padding(bottom = BottomPadding),
        ) {
          Column(
            modifier = Modifier.width(totalWidth),
          ) {
            Box(
              modifier = Modifier
                .width(totalWidth)
                .height(LabelHeight + 12.dp),
            ) {
              for ((index, span) in monthSpans.withIndex()) {
                if (span.end - span.start < 1 && index != monthSpans.lastIndex) continue

                val spanWidth =
                  (CellSize * (span.end - span.start + 1)) + (CellGap * (span.end - span.start))

                Text(
                  text = "${span.month}월",
                  modifier = Modifier
                    .align(Alignment.BottomStart)
                    .width(spanWidth)
                    .padding(end = 4.dp)
                    .then(Modifier.clickable { scrollToMonth(span) })
                    .offset(x = (CellSize + CellGap) * span.start),
                  style = AppTheme.typography.micro.copy(fontWeight = FontWeight.W500),
                  color = AppTheme.colors.textFaint,
                  maxLines = 1,
                )
              }
            }

            Spacer(Modifier.height(CellGap))

            Box(
              modifier = Modifier
                .width(totalWidth)
                .height(GridHeight)
                .then(gridGestureModifier),
            ) {
              Canvas(
                modifier = Modifier.fillMaxSize(),
              ) {
                val radiusPx = CellRadius.toPx()
                val selectionStrokePx = SelectionStrokeWidth.toPx()
                val selectedWeekIndex = selectedCell?.weekIndex
                val selectedDayIndex = selectedCell?.dayIndex

                weeks.forEachIndexed { weekIndex, week ->
                  week.forEachIndexed { dayIndex, activity ->
                    if (activity.level < 0) return@forEachIndexed

                    val topLeft = Offset(
                      x = weekIndex * (CellSize.toPx() + CellGap.toPx()),
                      y = dayIndex * (CellSize.toPx() + CellGap.toPx()),
                    )

                    drawRoundRect(
                      color = levelColors[activity.level.coerceIn(0, levelColors.lastIndex)],
                      topLeft = topLeft,
                      size = androidx.compose.ui.geometry.Size(CellSize.toPx(), CellSize.toPx()),
                      cornerRadius = CornerRadius(radiusPx, radiusPx),
                    )

                    if (selectedWeekIndex == weekIndex && selectedDayIndex == dayIndex) {
                      drawRoundRect(
                        color = selectionBorderColor,
                        topLeft = Offset(topLeft.x - 1.5f, topLeft.y - 1.5f),
                        size = androidx.compose.ui.geometry.Size(
                          CellSize.toPx() + 3f,
                          CellSize.toPx() + 3f
                        ),
                        cornerRadius = CornerRadius(radiusPx + 1.5f, radiusPx + 1.5f),
                        style = androidx.compose.ui.graphics.drawscope.Stroke(width = selectionStrokePx),
                      )
                    }
                  }
                }
              }
            }
          }
        }

        Box(
          modifier = Modifier
            .align(Alignment.CenterStart)
            .fillMaxHeight()
            .width(36.dp),
        ) {
          Box(
            modifier = Modifier
              .fillMaxSize()
              .graphicsLayer { alpha = leftArrowAlpha }
              .background(
                brush = Brush.horizontalGradient(
                  colorStops = arrayOf(
                    0.3f to AppTheme.colors.surfaceDefault.copy(alpha = 0.8f),
                    1f to AppTheme.colors.surfaceDefault.copy(alpha = 0f),
                  ),
                ),
              )
              .then(if (canScrollLeft) Modifier.clickable { scrollByMonths(-2) } else Modifier),
          ) {
            Icon(
              icon = Lucide.ChevronLeft,
              modifier = Modifier
                .align(Alignment.Center)
                .padding(horizontal = 8.dp)
                .width(20.dp),
              tint = AppTheme.colors.textSubtle,
            )
          }
        }

        Box(
          modifier = Modifier
            .align(Alignment.CenterEnd)
            .fillMaxHeight()
            .width(36.dp),
        ) {
          Box(
            modifier = Modifier
              .fillMaxSize()
              .graphicsLayer { alpha = rightArrowAlpha }
              .background(
                brush = Brush.horizontalGradient(
                  colorStops = arrayOf(
                    0f to AppTheme.colors.surfaceDefault.copy(alpha = 0f),
                    0.7f to AppTheme.colors.surfaceDefault.copy(alpha = 0.8f),
                  ),
                ),
              )
              .then(if (canScrollRight) Modifier.clickable { scrollByMonths(2) } else Modifier),
          ) {
            Icon(
              icon = Lucide.ChevronRight,
              modifier = Modifier
                .align(Alignment.Center)
                .padding(horizontal = 8.dp)
                .width(20.dp),
              tint = AppTheme.colors.textSubtle,
            )
          }
        }
      }

      val tooltipAlpha by animateFloatAsState(
        targetValue = if (tooltipVisible) 1f else 0f,
        animationSpec = tween(140),
      )

      if (tooltipData != null) {
        val tooltip = tooltipData!!
        Layout(
          modifier = Modifier
            .align(Alignment.TopStart)
            .fillMaxSize(),
          content = {
            Box(
              modifier = Modifier
                .alpha(tooltipAlpha)
                .clip(TooltipShape)
                .background(AppTheme.colors.surfaceDark)
                .padding(horizontal = 12.dp, vertical = 8.dp),
            ) {
              Column(
                verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(2.dp),
              ) {
                Text(
                  text = formatTooltipDate(tooltip.activity.date),
                  style = AppTheme.typography.micro.copy(
                    fontSize = 12.sp,
                    lineHeight = 16.sp,
                    fontWeight = FontWeight.W500
                  ),
                  color = AppTheme.colors.textBright,
                )
                Text(
                  text = if (tooltip.activity.additions > 0) {
                    "${tooltip.activity.additions.formatComma()}자 작성했어요"
                  } else {
                    "기록이 없어요"
                  },
                  style = AppTheme.typography.micro.copy(
                    fontSize = 12.sp,
                    lineHeight = 16.sp,
                    fontWeight = FontWeight.W700
                  ),
                  color = AppTheme.colors.textBright,
                )
              }
            }
          },
        ) { measurables, constraints ->
          val placeable = measurables.first().measure(constraints.copy(minWidth = 0, minHeight = 0))
          val tooltipOffset = calculateActivityGridTooltipOffset(
            cellOffset = Offset(
              x = with(density) { HorizontalPadding.toPx() } +
                (tooltip.weekIndex * cellStridePx) -
                scrollState.value,
              y = with(density) { (LabelHeight + CellGap).toPx() } +
                (tooltip.dayIndex * cellStridePx),
            ),
            tooltipWidthPx = placeable.width.toFloat(),
            tooltipHeightPx = placeable.height.toFloat(),
            cellSizePx = CellSizePx(density),
          )

          layout(constraints.maxWidth, constraints.maxHeight) {
            placeable.placeRelative(
              tooltipOffset.x.roundToInt(),
              tooltipOffset.y.roundToInt(),
            )
          }
        }
      }
    }
  }
}

private data class ActivityGridActivity(
  val date: LocalDate,
  val additions: Int,
  val level: Int,
)

private data class ActivityGridMonthSpan(
  val month: Int,
  val start: Int,
  val end: Int,
)

private data class ActivityGridSelection(
  val activity: ActivityGridActivity,
  val weekIndex: Int,
  val dayIndex: Int,
)

private sealed interface ActivityGridPreGestureResult {
  data class Tap(val position: Offset) : ActivityGridPreGestureResult
  object Cancel : ActivityGridPreGestureResult
}

private val CellSize = 13.dp
private val CellGap = 3.dp
private val GridHeight = (CellSize * 7) + (CellGap * 6)
private val LabelHeight = 16.dp
private val HorizontalPadding = 16.dp
private val BottomPadding = 8.dp
private val GridContainerHeight = LabelHeight + 12.dp + CellGap + GridHeight + BottomPadding
private val CellRadius = 2.dp
private val SelectionStrokeWidth = 1.5.dp
private val TooltipShape = RoundedCornerShape(6.dp)
private const val TooltipMinXPx = 8f
private const val TooltipHorizontalGapPx = 2f
private const val TooltipVerticalAdjustPx = 2f
private const val ScrollEdgeVisibilityThresholdPx = 1

val ActivityGridHeight = GridContainerHeight

private fun generateActivities(
  changes: List<ActivityGridChange>,
  startDate: LocalDate,
  endDate: LocalDate,
): List<ActivityGridActivity> {
  val changesByDate = changes.associate { it.date.toEpochDays() to it.additions }
  val positiveAdditions = changes.map { it.additions }.filter { it > 0 }
  val p95 = if (positiveAdditions.isEmpty()) {
    0
  } else {
    val sorted = positiveAdditions.sorted()
    val index = floor(sorted.size * 0.95f).toInt().coerceIn(0, sorted.lastIndex)
    sorted[index]
  }

  return buildList {
    for (offset in 1..((startDate.dayOfWeek.ordinal + 1) % 7)) {
      add(
        ActivityGridActivity(
          date = LocalDate.fromEpochDays(startDate.toEpochDays() - offset),
          additions = -1,
          level = -1,
        ),
      )
    }

    var currentDate = startDate
    while (currentDate <= endDate) {
      val additions = changesByDate[currentDate.toEpochDays()] ?: 0
      val level = when {
        additions == 0 -> 0
        p95 == 0 -> 3
        additions >= p95 -> 5
        else -> (((additions.toFloat() / p95) * 4).toInt() + 1).coerceAtMost(4)
      }

      add(
        ActivityGridActivity(
          date = currentDate,
          additions = additions,
          level = level,
        ),
      )

      currentDate = LocalDate.fromEpochDays(currentDate.toEpochDays() + 1)
    }
  }
}

internal fun calculateActivityGridTooltipOffset(
  cellOffset: Offset,
  tooltipWidthPx: Float,
  tooltipHeightPx: Float,
  cellSizePx: Float,
  horizontalGapPx: Float = TooltipHorizontalGapPx,
  verticalAdjustPx: Float = TooltipVerticalAdjustPx,
): Offset {
  return Offset(
    x = (cellOffset.x - tooltipWidthPx - horizontalGapPx).coerceAtLeast(TooltipMinXPx),
    y = cellOffset.y - tooltipHeightPx + cellSizePx - verticalAdjustPx,
  )
}

private fun generateWeeks(activities: List<ActivityGridActivity>): List<List<ActivityGridActivity>> {
  val weeks = mutableListOf<List<ActivityGridActivity>>()
  var index = 0

  while (index < activities.size) {
    weeks += activities.subList(index, minOf(index + 7, activities.size))
    index += 7
  }

  return weeks
}

private fun generateMonthSpans(activities: List<ActivityGridActivity>): List<ActivityGridMonthSpan> {
  val monthSpans = mutableListOf<ActivityGridMonthSpan>()
  val weekCount = activities.size / 7 + if (activities.size % 7 == 0) 0 else 1

  var previousMonth = -1
  var monthStartWeek = -1

  for (weekIndex in 0 until weekCount) {
    var weekMonth = -1
    var hasFirstOfMonth = false

    for (dayIndex in 0 until 7) {
      val activityIndex = weekIndex * 7 + dayIndex
      if (activityIndex >= activities.size) break

      val activity = activities[activityIndex]
      if (activity.level == -1) continue

      if (weekMonth == -1) {
        weekMonth = activity.date.month.number
      }

      if (activity.date.day == 1) {
        hasFirstOfMonth = true
        weekMonth = activity.date.month.number
        break
      }
    }

    if (weekIndex == 0 || (hasFirstOfMonth && weekMonth != previousMonth)) {
      if (monthStartWeek >= 0 && previousMonth != -1) {
        monthSpans += ActivityGridMonthSpan(
          month = previousMonth,
          start = monthStartWeek,
          end = weekIndex - 1,
        )
      }

      monthStartWeek = weekIndex
      previousMonth = weekMonth
    }
  }

  if (monthStartWeek >= 0 && previousMonth != -1) {
    monthSpans += ActivityGridMonthSpan(
      month = previousMonth,
      start = monthStartWeek,
      end = weekCount - 1,
    )
  }

  return monthSpans
}

private fun activityLevelColors(isDark: Boolean): List<Color> {
  return if (isDark) {
    listOf(
      AppColor.dark.gray.s800,
      AppColor.dark.green.s700,
      AppColor.dark.green.s500,
      AppColor.dark.green.s400,
      AppColor.dark.green.s300,
      AppColor.dark.green.s200,
    )
  } else {
    listOf(
      AppColor.light.gray.s200,
      AppColor.light.green.s300,
      AppColor.light.green.s500,
      AppColor.light.green.s600,
      AppColor.light.green.s700,
      AppColor.light.green.s800,
    )
  }
}

private fun CellSizePx(density: androidx.compose.ui.unit.Density): Float =
  with(density) { CellSize.toPx() }

private fun formatTooltipDate(date: LocalDate): String =
  "${date.year}년 ${date.month.number}월 ${date.day}일"

private fun Int.formatComma(): String {
  val raw = toString()
  val builder = StringBuilder(raw.length + (raw.length / 3))

  raw.forEachIndexed { index, char ->
    if (index > 0 && (raw.length - index) % 3 == 0) {
      builder.append(',')
    }
    builder.append(char)
  }

  return builder.toString()
}
