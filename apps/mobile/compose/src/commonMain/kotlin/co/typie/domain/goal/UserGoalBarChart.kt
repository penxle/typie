package co.typie.domain.goal

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Text
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.LocalDate
import kotlinx.datetime.minus

const val USER_GOAL_TREND_DAYS: Int = 28
const val USER_GOAL_BAR_Y_HEADROOM: Double = 1.05

data class DailyAdditionRow(val date: LocalDate, val additions: Long, val achieved: Boolean?)

fun dailyAdditionRows(
  changes: Map<LocalDate, Long>,
  judgments: Map<LocalDate, Boolean>,
  today: LocalDate,
  days: Int,
): List<DailyAdditionRow> =
  List(days) { index ->
    val date = today.minus(index, DateTimeUnit.DAY)

    DailyAdditionRow(date = date, additions = changes[date] ?: 0, achieved = judgments[date])
  }

fun userGoalBarYMax(rows: List<DailyAdditionRow>, target: Long?): Double =
  maxOf(1L, target ?: 0L, rows.maxOfOrNull { it.additions } ?: 0L) * USER_GOAL_BAR_Y_HEADROOM

@Composable
fun UserGoalBarChart(rows: List<DailyAdditionRow>, target: Long?, modifier: Modifier = Modifier) {
  if (rows.isEmpty()) {
    return
  }

  val series = rows.asReversed()
  val yMax = remember(rows, target) { userGoalBarYMax(rows, target) }

  val themeMode = AppTheme.themeMode
  val barColor =
    if (themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s600 else AppColor.light.gray.s400
  val targetColor =
    if (themeMode == ResolvedThemeMode.Dark) AppColor.dark.gray.s700 else AppColor.light.gray.s300

  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(BarSectionGap)) {
    Canvas(modifier = Modifier.fillMaxWidth().height(BarChartHeight)) {
      val gap = BarGap.toPx()
      val barWidth = (size.width - (series.size - 1) * gap) / series.size

      if (target != null) {
        val targetY = size.height - (target / yMax).toFloat() * size.height

        drawLine(
          color = targetColor,
          start = Offset(0f, targetY),
          end = Offset(size.width, targetY),
          strokeWidth = BarHairlineWidth.toPx(),
          pathEffect =
            PathEffect.dashPathEffect(
              floatArrayOf(BarTargetDashOn.toPx(), BarTargetDashOff.toPx())
            ),
        )
      }

      series.forEachIndexed { index, row ->
        if (row.additions > 0) {
          val barHeight = (row.additions / yMax).toFloat() * size.height

          drawRoundRect(
            color = barColor,
            topLeft = Offset(index * (barWidth + gap), size.height - barHeight),
            size = Size(barWidth, barHeight),
            cornerRadius = CornerRadius(BarRadius.toPx()),
          )
        }
      }
    }

    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
      Text(
        text = goalMonthDayLabel(series.first().date),
        style = AppTheme.typography.micro,
        color = AppTheme.colors.textHint,
      )

      Text(
        text = goalMonthDayLabel(series.last().date) + if (target == null) "" else " · ┄ 목표선",
        style = AppTheme.typography.micro,
        color = AppTheme.colors.textHint,
      )
    }
  }
}

private val BarChartHeight = 80.dp
private val BarGap = 3.dp
private val BarRadius = 1.dp
private val BarHairlineWidth = 1.dp
private val BarSectionGap = 4.dp
private val BarTargetDashOn = 4.dp
private val BarTargetDashOff = 3.dp
