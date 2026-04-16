package co.typie.domain.stats

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.datetime.toLocalDate
import co.typie.graphql.fragment.ActivityGrid_user
import co.typie.ui.component.ScrollFogInsets
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.component.toPaddingValues
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppColor
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.time.Clock
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

  Row(
    modifier =
      modifier
        .scrollFog(FogInsets, AppTheme.colors.surfaceDefault)
        .horizontalScroll(scrollState)
        .padding(FogInsets.toPaddingValues()),
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
                Modifier.size(CellSize).background(colors[activity.level], RoundedCornerShape(2.dp))
            )
          } else {
            Spacer(modifier = Modifier.size(CellSize))
          }
        }
      }
    }
  }
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
private val FogInsets = ScrollFogInsets(left = 16.dp, right = 16.dp)
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
