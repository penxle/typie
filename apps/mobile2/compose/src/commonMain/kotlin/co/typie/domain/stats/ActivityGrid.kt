package co.typie.domain.stats

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.datetime.toLocalDate
import co.typie.ext.comma
import co.typie.graphql.fragment.ActivityGrid_user
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.time.Clock
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.LocalDate
import kotlinx.datetime.isoDayNumber
import kotlinx.datetime.minus
import kotlinx.datetime.number
import kotlinx.datetime.plus

@Composable
fun ActivityGrid(user: ActivityGrid_user, modifier: Modifier = Modifier) {
  val scrollState = rememberScrollState(initial = Int.MAX_VALUE)

  val themeMode = AppTheme.themeMode
  val colors = remember(themeMode) { activityLevelColors(themeMode) }

  val endDate = remember { Clock.System.now().toLocalDate() }
  val startDate = remember { endDate.minus(364, DateTimeUnit.DAY) }
  val gridDates = remember { gridDates(startDate, endDate) }
  val activities =
    remember(user.characterCountChanges) {
      computeActivities(user.characterCountChanges, startDate, endDate)
    }
  val weeks = remember(activities) { computeWeeks(activities, gridDates, startDate, endDate) }
  val monthLabelByWeek = remember { computeMonthLabels(gridDates, startDate, endDate) }

  val density = LocalDensity.current
  val layoutDirection = LocalLayoutDirection.current
  val haptic = LocalHapticFeedback.current
  val cellStridePx = with(density) { (CellSize + CellGap).toPx() }
  val cellSizePx = with(density) { CellSize.toPx() }
  val fogLeftPx = with(density) { FogInsets.calculateLeftPadding(layoutDirection).toPx() }
  val fogRightPx = with(density) { FogInsets.calculateRightPadding(layoutDirection).toPx() }
  val monthLabelHeightPx = with(density) { MonthLabelHeight.toPx() }
  val cellGapPx = with(density) { CellGap.toPx() }
  val tooltipOffsetPx = with(density) { TooltipOffset.toPx() }

  var viewportWidthPx by remember { mutableStateOf(0) }
  var activeCell by remember { mutableStateOf<ActiveCell?>(null) }
  var pressed by remember { mutableStateOf(false) }
  var displayCell by remember { mutableStateOf<ActiveCell?>(null) }
  val alpha = remember { Animatable(0f) }
  LaunchedEffect(activeCell) {
    activeCell?.let {
      displayCell = it
      haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
    }
  }
  LaunchedEffect(activeCell, pressed) {
    if (activeCell != null) {
      alpha.animateTo(1f, tween(FadeDurationMs))
      if (!pressed) {
        delay(LingerMs)
        activeCell = null
      }
    } else {
      alpha.animateTo(0f, tween(FadeDurationMs))
      displayCell = null
    }
  }

  Box(modifier = modifier.onSizeChanged { viewportWidthPx = it.width }) {
    Row(
      modifier =
        Modifier.scrollFog(FogInsets, AppTheme.colors.surfaceDefault)
          .horizontalScroll(scrollState)
          .padding(FogInsets)
          .pointerInput(weeks) {
            val slopPx = ArmSlop.toPx()
            val slopSquared = slopPx * slopPx
            awaitPointerEventScope {
              while (true) {
                val down = awaitFirstDown(requireUnconsumed = true)
                val originX = down.position.x
                val originY = down.position.y
                var lastX = originX
                var lastY = originY

                var cancelled = false
                val longPress =
                  withTimeoutOrNull(ArmDelayMs) {
                    while (true) {
                      val event = awaitPointerEvent()
                      val change =
                        event.changes.firstOrNull { it.id == down.id } ?: return@withTimeoutOrNull
                      lastX = change.position.x
                      lastY = change.position.y
                      val dx = lastX - originX
                      val dy = lastY - originY
                      if (dx * dx + dy * dy > slopSquared) {
                        cancelled = true
                        return@withTimeoutOrNull
                      }
                      if (!change.pressed) return@withTimeoutOrNull
                    }
                  } == null

                if (cancelled) continue

                activeCell =
                  computeActiveCell(
                    pointerX = lastX,
                    pointerY = lastY,
                    scrollOffsetPx = scrollState.value.toFloat(),
                    viewportWidthPx = viewportWidthPx.toFloat(),
                    fogLeftPx = fogLeftPx,
                    fogRightPx = fogRightPx,
                    cellStridePx = cellStridePx,
                    monthLabelHeightPx = monthLabelHeightPx,
                    cellGapPx = cellGapPx,
                    weeks = weeks,
                  )

                if (longPress) {
                  pressed = true
                  while (true) {
                    val event = awaitPointerEvent()
                    val change = event.changes.firstOrNull { it.id == down.id } ?: break
                    change.consume()
                    activeCell =
                      computeActiveCell(
                        pointerX = change.position.x,
                        pointerY = change.position.y,
                        scrollOffsetPx = scrollState.value.toFloat(),
                        viewportWidthPx = viewportWidthPx.toFloat(),
                        fogLeftPx = fogLeftPx,
                        fogRightPx = fogRightPx,
                        cellStridePx = cellStridePx,
                        monthLabelHeightPx = monthLabelHeightPx,
                        cellGapPx = cellGapPx,
                        weeks = weeks,
                      )
                    if (!change.pressed) break
                  }
                  pressed = false
                }
              }
            }
          },
      horizontalArrangement = Arrangement.spacedBy(CellGap),
    ) {
      Column(verticalArrangement = Arrangement.spacedBy(CellGap)) {
        Spacer(modifier = Modifier.height(MonthLabelHeight))

        Weekdays.forEach { weekday ->
          Box(modifier = Modifier.size(CellSize), contentAlignment = Alignment.CenterStart) {
            if (weekday != null) {
              Text(
                text = weekday,
                style = TextStyle(fontSize = 10.sp, fontWeight = FontWeight.Medium),
                color = AppTheme.colors.textTertiary,
              )
            }
          }
        }
      }

      weeks.forEachIndexed { weekIndex, week ->
        Column(verticalArrangement = Arrangement.spacedBy(CellGap)) {
          Box(
            modifier = Modifier.size(width = CellSize, height = MonthLabelHeight),
            contentAlignment = Alignment.BottomStart,
          ) {
            monthLabelByWeek[weekIndex]?.let { label ->
              Text(
                text = label,
                style = TextStyle(fontSize = 10.sp, fontWeight = FontWeight.Medium),
                color = AppTheme.colors.textTertiary,
                softWrap = false,
                overflow = TextOverflow.Visible,
              )
            }
          }

          week.forEach { activity ->
            if (activity != null) {
              Box(
                modifier =
                  Modifier.size(CellSize)
                    .background(colors[activity.level], RoundedCornerShape(2.dp))
              )
            } else {
              Spacer(modifier = Modifier.size(CellSize))
            }
          }
        }
      }
    }

    displayCell?.let { cell ->
      val activity = weeks.getOrNull(cell.weekIndex)?.getOrNull(cell.dayIndex) ?: return@let
      val anchorContentX =
        fogLeftPx + cellStridePx + cell.weekIndex * cellStridePx + cellSizePx / 2f
      val anchorViewportX = anchorContentX - scrollState.value
      val anchorY = monthLabelHeightPx + cellGapPx + cell.dayIndex * cellStridePx

      Layout(
        content = {
          ActivityTooltip(date = activity.date, additions = activity.additions, alpha = alpha.value)
        }
      ) { measurables, constraints ->
        val placeable = measurables.first().measure(Constraints())
        val x =
          (anchorViewportX - placeable.width / 2f)
            .toInt()
            .coerceIn(0, constraints.maxWidth - placeable.width)
        val y = (anchorY - placeable.height - tooltipOffsetPx).toInt()
        layout(0, 0) { placeable.place(x, y) }
      }
    }
  }
}

@Composable
private fun ActivityTooltip(date: LocalDate, additions: Int, alpha: Float) {
  val themeMode = AppTheme.themeMode
  val background =
    if (themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s500 else AppColor.light.gray.s600

  val tooltipShape = RoundedCornerShape(6.dp)
  Column(
    modifier =
      Modifier.graphicsLayer {
          this.alpha = alpha
          shape = tooltipShape
          clip = true
        }
        .background(background)
        .padding(horizontal = 10.dp, vertical = 6.dp)
  ) {
    Text(
      text = "${date.year}년 ${date.month.number}월 ${date.day}일",
      style = TextStyle(fontSize = 12.sp, fontWeight = FontWeight.Medium),
      color = AppColor.white,
    )
    Text(
      text = if (additions > 0) "${additions.comma}자 작성했어요" else "기록이 없어요",
      style = TextStyle(fontSize = 12.sp, fontWeight = FontWeight.Bold),
      color = AppColor.white,
    )
  }
}

private data class ActiveCell(val weekIndex: Int, val dayIndex: Int)

private fun computeActiveCell(
  pointerX: Float,
  pointerY: Float,
  scrollOffsetPx: Float,
  viewportWidthPx: Float,
  fogLeftPx: Float,
  fogRightPx: Float,
  cellStridePx: Float,
  monthLabelHeightPx: Float,
  cellGapPx: Float,
  weeks: List<List<Activity?>>,
): ActiveCell? {
  if (viewportWidthPx <= 0f) return null
  val viewportX = pointerX - scrollOffsetPx + fogLeftPx
  if (viewportX < fogLeftPx || viewportX > viewportWidthPx - fogRightPx) return null

  val xInWeeks = pointerX - cellStridePx
  if (xInWeeks < 0f) return null
  val weekIndex = (xInWeeks / cellStridePx).toInt()
  if (weekIndex !in weeks.indices) return null

  val yInGrid = pointerY - monthLabelHeightPx - cellGapPx
  if (yInGrid < 0f) return null
  val dayIndex = (yInGrid / cellStridePx).toInt()
  if (dayIndex !in 0 until 7) return null

  weeks[weekIndex].getOrNull(dayIndex) ?: return null
  return ActiveCell(weekIndex, dayIndex)
}

private fun activityLevelColors(themeMode: ResolvedThemeMode): List<Color> {
  return when (themeMode) {
    ResolvedThemeMode.Dark ->
      listOf(
        AppColor.dark.gray.s800,
        AppColor.dark.green.s700,
        AppColor.dark.green.s500,
        AppColor.dark.green.s400,
        AppColor.dark.green.s300,
        AppColor.dark.green.s200,
      )
    ResolvedThemeMode.Light ->
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

private val CellSize = 12.dp
private val CellGap = 3.dp
private val MonthLabelHeight = 16.dp
private val FogInsets = PaddingValues(horizontal = 16.dp)
private val TooltipOffset = 24.dp
private val ArmSlop = 6.dp
private const val ArmDelayMs = 300L
private const val LingerMs = 500L
private const val FadeDurationMs = 250
private val Weekdays = listOf(null, "월", null, "수", null, "금", null)

private data class Activity(val date: LocalDate, val additions: Int, val level: Int)

private data class MonthSpan(val month: Int, val startWeek: Int, val endWeek: Int)

private fun gridDates(startDate: LocalDate, endDate: LocalDate): List<LocalDate> {
  val startDayIndex = startDate.dayOfWeek.isoDayNumber % 7
  val endDayIndex = endDate.dayOfWeek.isoDayNumber % 7
  val gridStart = startDate.minus(startDayIndex, DateTimeUnit.DAY)
  val gridEnd = endDate.plus(6 - endDayIndex, DateTimeUnit.DAY)
  return generateSequence(gridStart) { it.plus(1, DateTimeUnit.DAY) }
    .takeWhile { it <= gridEnd }
    .toList()
}

private fun computeActivities(
  changes: List<ActivityGrid_user.CharacterCountChange>,
  startDate: LocalDate,
  endDate: LocalDate,
): List<Activity> {
  val sortedPositives = changes.map { it.additions }.filter { it > 0 }.sorted()
  val p95 =
    sortedPositives.getOrElse(
      (sortedPositives.size * 0.95).toInt().coerceAtMost(sortedPositives.lastIndex)
    ) {
      0
    }

  val additionsByDate = changes.associate { it.date.toLocalDate() to it.additions }

  return generateSequence(startDate) { it.plus(1, DateTimeUnit.DAY) }
    .takeWhile { it <= endDate }
    .map { date ->
      val additions = additionsByDate[date] ?: 0
      val level =
        when {
          additions == 0 -> 0
          p95 == 0 -> 3
          additions >= p95 -> 5
          else -> minOf((additions.toFloat() / p95 * 4).toInt() + 1, 4)
        }
      Activity(date = date, additions = additions, level = level)
    }
    .toList()
}

private fun computeWeeks(
  activities: List<Activity>,
  gridDates: List<LocalDate>,
  startDate: LocalDate,
  endDate: LocalDate,
): List<List<Activity?>> {
  val activityByDate = activities.associateBy { it.date }
  return gridDates.chunked(7).map { week ->
    week.map { if (it in startDate..endDate) activityByDate[it] else null }
  }
}

private fun computeMonthLabels(
  gridDates: List<LocalDate>,
  startDate: LocalDate,
  endDate: LocalDate,
): Map<Int, String> {
  val weekMonths =
    gridDates.chunked(7).map { week ->
      val valid = week.filter { it in startDate..endDate }
      val representative = valid.firstOrNull { it.day == 1 } ?: valid.firstOrNull()
      representative?.month?.number
    }

  val spans = buildList {
    var startWeek = 0
    var currentMonth = -1
    weekMonths.forEachIndexed { i, month ->
      if (month != null && month != currentMonth) {
        if (currentMonth != -1) add(MonthSpan(currentMonth, startWeek, i - 1))
        currentMonth = month
        startWeek = i
      }
    }
    if (currentMonth != -1) add(MonthSpan(currentMonth, startWeek, weekMonths.lastIndex))
  }

  return spans
    .filterIndexed { i, span -> span.endWeek - span.startWeek >= 1 || i == spans.lastIndex }
    .associate { it.startWeek to "${it.month}월" }
}
