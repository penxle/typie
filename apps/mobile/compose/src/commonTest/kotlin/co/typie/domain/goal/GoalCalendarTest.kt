package co.typie.domain.goal

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.datetime.LocalDate

class GoalCalendarTest {
  @Test
  fun august2026StartsOnSaturday() {
    val grid = monthGridDates(2026, 8)
    assertEquals(6, grid.takeWhile { it == null }.count())
    assertEquals(LocalDate(2026, 8, 1), grid[6])
    assertEquals(LocalDate(2026, 8, 31), grid.last())
    assertEquals(37, grid.size)
  }

  @Test
  fun sundayStartHasNoPadding() {
    val grid = monthGridDates(2026, 11)
    assertEquals(LocalDate(2026, 11, 1), grid.first())
  }

  @Test
  fun weeksAlwaysSixRowsOfSeven() {
    listOf(2026 to 2, 2026 to 9, 2026 to 8).forEach { (year, month) ->
      val weeks = monthGridWeeks(year, month)
      assertEquals(6, weeks.size)
      assertTrue(weeks.all { it.size == 7 })
    }
  }

  @Test
  fun weeksPreserveGridDatesThenPadWithNulls() {
    listOf(2026 to 2, 2026 to 9, 2026 to 8).forEach { (year, month) ->
      val grid = monthGridDates(year, month)
      val cells = monthGridWeeks(year, month).flatten()
      assertEquals(grid, cells.take(grid.size))
      assertTrue(cells.drop(grid.size).all { it == null })
    }
  }

  @Test
  fun narrowWidthShrinksPaddingUntilCellsReachMinimumSize() {
    val padding = goalCalendarHorizontalPadding(320.dp)

    assertEquals(6.dp, padding)
    assertEquals(GoalCalendarMinCellSize, (320.dp - padding * 2) / GOAL_CALENDAR_COLUMNS)
  }

  @Test
  fun regularWidthsKeepDefaultPadding() {
    listOf(340.dp, 375.dp, 393.dp, 430.dp).forEach { width ->
      assertEquals(GoalCalendarMaxHorizontalPadding, goalCalendarHorizontalPadding(width))
    }
  }

  @Test
  fun paddingNeverFallsBelowFloorOnExtremelyNarrowWidths() {
    assertEquals(GoalCalendarMinHorizontalPadding, goalCalendarHorizontalPadding(280.dp))
  }
}
