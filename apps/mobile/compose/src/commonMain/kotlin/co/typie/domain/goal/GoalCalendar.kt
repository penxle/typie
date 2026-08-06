package co.typie.domain.goal

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.LocalDate
import kotlinx.datetime.isoDayNumber
import kotlinx.datetime.minus
import kotlinx.datetime.plus

const val GOAL_CALENDAR_COLUMNS: Int = 7
const val GOAL_CALENDAR_ROWS: Int = 6

val GoalCalendarMinCellSize: Dp = 44.dp
val GoalCalendarMaxHorizontalPadding: Dp = 16.dp
val GoalCalendarMinHorizontalPadding: Dp = 4.dp

fun goalCalendarHorizontalPadding(width: Dp): Dp =
  ((width - GoalCalendarMinCellSize * GOAL_CALENDAR_COLUMNS) / 2).coerceIn(
    GoalCalendarMinHorizontalPadding,
    GoalCalendarMaxHorizontalPadding,
  )

fun monthGridDates(year: Int, month: Int): List<LocalDate?> {
  val first = LocalDate(year, month, 1)
  val last = first.plus(1, DateTimeUnit.MONTH).minus(1, DateTimeUnit.DAY)
  val padding = first.dayOfWeek.isoDayNumber % 7

  return List(padding) { null } + (1..last.day).map { LocalDate(year, month, it) }
}

fun monthGridWeeks(year: Int, month: Int): List<List<LocalDate?>> {
  val cells = monthGridDates(year, month)
  val total = GOAL_CALENDAR_COLUMNS * GOAL_CALENDAR_ROWS

  return (cells + List(total - cells.size) { null }).chunked(GOAL_CALENDAR_COLUMNS)
}
