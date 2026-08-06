package co.typie.domain.goal

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.datetime.WeekdayNames
import co.typie.ext.comma
import co.typie.ui.component.Text
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.math.ceil
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.LocalDate
import kotlinx.datetime.isoDayNumber
import kotlinx.datetime.minus

const val USER_GOAL_DOT_DAYS: Int = 112

data class DotDay(val date: LocalDate, val additions: Long?, val achieved: Boolean?)

fun dotDays(history: List<UserGoalDay>, today: LocalDate): List<DotDay> {
  val byDate = history.associateBy { it.date }

  return List(USER_GOAL_DOT_DAYS) { index ->
    val date = today.minus(USER_GOAL_DOT_DAYS - 1 - index, DateTimeUnit.DAY)
    val row = byDate[date]

    DotDay(date = date, additions = row?.additions, achieved = row?.achieved)
  }
}

fun dotDayMessage(day: DotDay): String {
  val weekday = WeekdayNames[day.date.dayOfWeek.isoDayNumber % WeekdayNames.size]
  val label = "${goalMonthDayLabel(day.date)} $weekday"
  val additions = day.additions ?: return "$label · 목표 없음"

  val state =
    when {
      day.achieved == true -> "달성"
      additions > 0 -> "일부 달성"
      else -> "미달성"
    }

  return "$label · ${additions.comma}자 · $state"
}

@Composable
fun UserGoalDots(days: List<DotDay>, modifier: Modifier = Modifier) {
  val themeMode = AppTheme.themeMode
  val neutral =
    if (themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s700 else AppColor.light.gray.s300
  val success = AppTheme.colors.success

  val density = LocalDensity.current
  val haptic = LocalHapticFeedback.current
  val metrics = remember(density) { dotMetrics(density) }
  val tooltipOffsetPx = with(density) { DotTooltipOffset.toPx() }

  var gridWidthPx by remember { mutableStateOf(0) }
  var active by remember { mutableStateOf<Int?>(null) }
  var display by remember { mutableStateOf<Int?>(null) }
  val alpha = remember { Animatable(0f) }

  val perRow = dotsPerRow(metrics, gridWidthPx)

  LaunchedEffect(active) {
    val current = active
    if (current == null) {
      alpha.animateTo(0f, tween(DOT_TOOLTIP_FADE_MILLIS))
      display = null
    } else {
      display = current
      haptic.performHapticFeedback(HapticFeedbackType.SegmentFrequentTick)
      alpha.animateTo(1f, tween(DOT_TOOLTIP_FADE_MILLIS))
    }
  }

  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(DotSectionGap)) {
    Box {
      FlowRow(
        modifier =
          Modifier.fillMaxWidth()
            .onSizeChanged { gridWidthPx = it.width }
            .pointerInput(days, perRow, metrics) {
              detectTapGestures { offset ->
                val index = dotIndexAt(offset, perRow, metrics.stridePx, days.size)
                active = if (index == active) null else index
              }
            },
        horizontalArrangement = Arrangement.spacedBy(DotGap),
        verticalArrangement = Arrangement.spacedBy(DotGap),
      ) {
        days.forEach { day ->
          GoalDot(state = dotState(day), size = DotSize, success = success, neutral = neutral)
        }
      }

      val index = display
      val day = index?.let { days.getOrNull(it) }
      if (index != null && day != null && perRow > 0) {
        val anchorX = (index % perRow) * metrics.stridePx + metrics.dotPx / 2f
        val anchorY = (index / perRow) * metrics.stridePx.toFloat()

        Layout(content = { DotTooltip(message = dotDayMessage(day), alpha = alpha.value) }) {
          measurables,
          constraints ->
          val placeable = measurables.first().measure(Constraints())
          val x =
            (anchorX - placeable.width / 2f)
              .toInt()
              .coerceIn(0, maxOf(0, constraints.maxWidth - placeable.width))
          val y = (anchorY - placeable.height - tooltipOffsetPx).toInt()
          layout(0, 0) { placeable.place(x, y) }
        }
      }
    }

    FlowRow(
      horizontalArrangement = Arrangement.spacedBy(DotLegendGap),
      verticalArrangement = Arrangement.spacedBy(DotLegendGap),
    ) {
      DotLegendItem(DotState.Achieved, "달성", success, neutral)
      DotLegendItem(DotState.Partial, "일부 달성", success, neutral)
      DotLegendItem(DotState.Missed, "미달성", success, neutral)
      DotLegendItem(DotState.NoGoal, "목표 없음", success, neutral)
    }
  }
}

internal enum class DotState {
  Achieved,
  Partial,
  Missed,
  NoGoal,
}

internal fun dotState(day: DotDay): DotState =
  when {
    day.additions == null -> DotState.NoGoal
    day.achieved == true -> DotState.Achieved
    day.additions > 0 -> DotState.Partial
    else -> DotState.Missed
  }

internal data class DotMetrics(val dotPx: Int, val stridePx: Int, val wrapGapPx: Int)

internal fun dotMetrics(density: Density): DotMetrics =
  with(density) {
    DotMetrics(
      dotPx = DotSize.roundToPx(),
      stridePx = DotSize.roundToPx() + DotGap.roundToPx(),
      wrapGapPx = ceil(DotGap.toPx()).toInt(),
    )
  }

internal fun dotsPerRow(metrics: DotMetrics, gridWidthPx: Int): Int {
  if (gridWidthPx <= 0) {
    return 0
  }

  return maxOf(1, (gridWidthPx + metrics.wrapGapPx) / (metrics.dotPx + metrics.wrapGapPx))
}

internal fun dotIndexAt(offset: Offset, perRow: Int, stridePx: Int, count: Int): Int? {
  if (perRow <= 0 || stridePx <= 0) {
    return null
  }

  if (offset.x < 0f || offset.y < 0f) {
    return null
  }

  val column = (offset.x / stridePx).toInt()
  if (column >= perRow) {
    return null
  }

  val index = (offset.y / stridePx).toInt() * perRow + column

  return if (index < count) index else null
}

@Composable
private fun GoalDot(state: DotState, size: Dp, success: Color, neutral: Color) {
  val base = Modifier.size(size)

  Box(
    modifier =
      when (state) {
        DotState.Achieved -> base.background(success, CircleShape)
        DotState.Partial -> base.background(success.copy(alpha = DOT_PARTIAL_ALPHA), CircleShape)
        DotState.Missed -> base.background(neutral, CircleShape)
        DotState.NoGoal -> base.border(DotBorderWidth, neutral, CircleShape)
      }
  )
}

@Composable
private fun DotLegendItem(state: DotState, label: String, success: Color, neutral: Color) {
  Row(
    horizontalArrangement = Arrangement.spacedBy(DotLegendItemGap),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    GoalDot(state = state, size = DotLegendSwatchSize, success = success, neutral = neutral)

    Text(text = label, style = AppTheme.typography.micro, color = AppTheme.colors.textHint)
  }
}

@Composable
private fun DotTooltip(message: String, alpha: Float) {
  val themeMode = AppTheme.themeMode
  val background =
    if (themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s500 else AppColor.light.gray.s600
  val tooltipShape = RoundedCornerShape(DotTooltipRadius)

  Box(
    modifier =
      Modifier.graphicsLayer {
          this.alpha = alpha
          shape = tooltipShape
          clip = true
        }
        .background(background)
        .padding(horizontal = DotTooltipPaddingHorizontal, vertical = DotTooltipPaddingVertical)
  ) {
    Text(
      text = message,
      style = TextStyle(fontSize = 12.sp, fontWeight = FontWeight.Medium),
      color = AppColor.white,
      softWrap = false,
    )
  }
}

private const val DOT_PARTIAL_ALPHA = 0.4f
private const val DOT_TOOLTIP_FADE_MILLIS = 250

private val DotSize = 10.dp
private val DotGap = 3.dp
private val DotBorderWidth = 1.dp
private val DotSectionGap = 8.dp
private val DotLegendGap = 10.dp
private val DotLegendItemGap = 4.dp
private val DotLegendSwatchSize = 8.dp
private val DotTooltipOffset = 24.dp
private val DotTooltipRadius = 6.dp
private val DotTooltipPaddingHorizontal = 10.dp
private val DotTooltipPaddingVertical = 6.dp
