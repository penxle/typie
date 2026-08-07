package co.typie.domain.goal

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.ui.component.Text
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import kotlinx.datetime.LocalDate
import kotlinx.datetime.daysUntil
import kotlinx.datetime.number

const val GOAL_TREND_Y_HEADROOM: Double = 1.05

data class CharacterCountPoint(val date: LocalDate, val characterCount: Long)

data class GoalTrendPace(val date: LocalDate, val value: Long)

data class GoalTrendScale(
  val first: LocalDate,
  val last: LocalDate,
  val spanDays: Int,
  val yMax: Double,
) {
  fun x(date: LocalDate, left: Float, plotWidth: Float): Float =
    left + (first.daysUntil(date).toFloat() / spanDays) * plotWidth

  fun y(value: Long, top: Float, plotHeight: Float): Float =
    top + (1.0 - value / yMax).toFloat() * plotHeight
}

fun goalTrendScale(
  history: List<CharacterCountPoint>,
  current: Long,
  goal: EntityGoalData?,
  today: LocalDate,
): GoalTrendScale {
  val first = history.firstOrNull()?.date ?: today
  val last = goal?.dueDate?.let { maxOf(today, it) } ?: today
  val peak =
    maxOf(
      1L,
      goal?.targetCharacterCount ?: 0L,
      current,
      history.maxOfOrNull { it.characterCount } ?: 0L,
    )

  return GoalTrendScale(
    first = first,
    last = last,
    spanDays = maxOf(1, first.daysUntil(last)),
    yMax = peak * GOAL_TREND_Y_HEADROOM,
  )
}

fun goalTrendPace(history: List<CharacterCountPoint>, goal: EntityGoalData?): GoalTrendPace? {
  if (goal?.dueDate == null) {
    return null
  }

  val created = goal.createdDate
  val anchor = history.lastOrNull { it.date <= created } ?: history.firstOrNull() ?: return null

  return GoalTrendPace(date = maxOf(created, anchor.date), value = anchor.characterCount)
}

fun goalMonthDayLabel(date: LocalDate): String = "${date.month.number}월 ${date.day}일"

@Composable
fun EntityGoalTrendChart(
  history: List<CharacterCountPoint>,
  current: Long,
  goal: EntityGoalData?,
  today: LocalDate,
  modifier: Modifier = Modifier,
) {
  if (history.isEmpty()) {
    GoalHistoryEmptyMessage(modifier)
    return
  }

  val scale =
    remember(history, current, goal, today) { goalTrendScale(history, current, goal, today) }
  val pace = remember(history, goal) { goalTrendPace(history, goal) }
  val target = goal?.targetCharacterCount
  val dueDate = goal?.dueDate

  val themeMode = AppTheme.themeMode
  val colors = remember(themeMode) { goalTrendColors(themeMode) }

  Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(GoalTrendSectionGap)) {
    Canvas(modifier = Modifier.fillMaxWidth().height(GoalTrendHeight)) {
      val left = GoalTrendPaddingLeft.toPx()
      val top = GoalTrendPaddingTop.toPx()
      val right = size.width - GoalTrendPaddingRight.toPx()
      val plotWidth = right - left
      val plotHeight = size.height - top - GoalTrendPaddingBottom.toPx()

      if (target != null) {
        val targetY = scale.y(target, top, plotHeight)

        drawLine(
          color = colors.target,
          start = Offset(left, targetY),
          end = Offset(right, targetY),
          strokeWidth = GoalTrendHairlineWidth.toPx(),
          pathEffect = dashEffect(GoalTrendTargetDash),
        )
      }

      if (target != null && dueDate != null && pace != null) {
        drawLine(
          color = colors.pace,
          start =
            Offset(
              scale.x(pace.date, left, plotWidth),
              scale.y(pace.value, top, plotHeight),
            ),
          end = Offset(scale.x(dueDate, left, plotWidth), scale.y(target, top, plotHeight)),
          strokeWidth = GoalTrendHairlineWidth.toPx(),
          pathEffect = dashEffect(GoalTrendPaceDash),
        )
      }

      val path = Path()
      history.forEachIndexed { index, point ->
        val pointX = scale.x(point.date, left, plotWidth)
        val pointY = scale.y(point.characterCount, top, plotHeight)

        if (index == 0) {
          path.moveTo(pointX, pointY)
        } else {
          path.lineTo(pointX, pointY)
        }
      }

      drawPath(
        path = path,
        color = colors.actual,
        style = Stroke(width = GoalTrendStrokeWidth.toPx(), join = StrokeJoin.Round),
      )

      drawCircle(
        color = colors.today,
        radius = GoalTrendDotRadius.toPx(),
        center = Offset(scale.x(today, left, plotWidth), scale.y(current, top, plotHeight)),
      )
    }

    Row(
      modifier =
        Modifier.fillMaxWidth().padding(start = GoalTrendPaddingLeft, end = GoalTrendPaddingRight),
      horizontalArrangement = Arrangement.SpaceBetween,
    ) {
      Text(
        text = goalMonthDayLabel(scale.first),
        style = AppTheme.typography.micro,
        color = AppTheme.colors.textHint,
      )

      Text(
        text = goalMonthDayLabel(scale.last),
        style = AppTheme.typography.micro,
        color = AppTheme.colors.textHint,
      )
    }

    Row(
      horizontalArrangement = Arrangement.spacedBy(GoalTrendLegendGap),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      GoalTrendLegendItem(
        color = colors.actual,
        strokeWidth = GoalTrendStrokeWidth,
        dash = null,
        label = "글자 수",
      )

      if (target != null) {
        GoalTrendLegendItem(
          color = colors.target,
          strokeWidth = GoalTrendHairlineWidth,
          dash = GoalTrendTargetDash,
          label = "목표선",
        )
      }

      if (dueDate != null && pace != null) {
        GoalTrendLegendItem(
          color = colors.pace,
          strokeWidth = GoalTrendHairlineWidth,
          dash = GoalTrendPaceDash,
          label = "필요 페이스",
        )
      }
    }
  }
}

@Composable
private fun GoalTrendLegendItem(
  color: Color,
  strokeWidth: Dp,
  dash: GoalTrendDash?,
  label: String,
) {
  Row(
    horizontalArrangement = Arrangement.spacedBy(GoalTrendLegendItemGap),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Canvas(modifier = Modifier.size(width = GoalTrendSwatchWidth, height = GoalTrendSwatchHeight)) {
      drawLine(
        color = color,
        start = Offset(0f, size.height / 2f),
        end = Offset(size.width, size.height / 2f),
        strokeWidth = strokeWidth.toPx(),
        pathEffect = dash?.let { dashEffect(it) },
      )
    }

    Text(text = label, style = AppTheme.typography.micro, color = AppTheme.colors.textHint)
  }
}

@Immutable private data class GoalTrendDash(val on: Dp, val off: Dp)

@Immutable
private data class GoalTrendColors(
  val actual: Color,
  val target: Color,
  val pace: Color,
  val today: Color,
)

private fun goalTrendColors(themeMode: ResolvedThemeMode): GoalTrendColors =
  if (themeMode == ResolvedThemeMode.Dark) {
    GoalTrendColors(
      actual = AppColor.dark.gray.s400,
      target = AppColor.dark.gray.s700,
      pace = AppColor.dark.gray.s600,
      today = AppColor.dark.gray.s300,
    )
  } else {
    GoalTrendColors(
      actual = AppColor.light.gray.s600,
      target = AppColor.light.gray.s300,
      pace = AppColor.light.gray.s400,
      today = AppColor.light.gray.s700,
    )
  }

private fun DrawScope.dashEffect(dash: GoalTrendDash): PathEffect =
  PathEffect.dashPathEffect(floatArrayOf(dash.on.toPx(), dash.off.toPx()))

private val GoalTrendHeight = 180.dp
private val GoalTrendPaddingTop = 8.dp
private val GoalTrendPaddingRight = 8.dp
private val GoalTrendPaddingBottom = 0.dp
private val GoalTrendPaddingLeft = 8.dp
private val GoalTrendSectionGap = 8.dp
private val GoalTrendStrokeWidth = 2.dp
private val GoalTrendHairlineWidth = 1.dp
private val GoalTrendDotRadius = 3.dp
private val GoalTrendLegendGap = 12.dp
private val GoalTrendLegendItemGap = 4.dp
private val GoalTrendSwatchWidth = 16.dp
private val GoalTrendSwatchHeight = 8.dp
private val GoalTrendTargetDash = GoalTrendDash(on = 4.dp, off = 3.dp)
private val GoalTrendPaceDash = GoalTrendDash(on = 2.dp, off = 3.dp)
