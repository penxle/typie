package co.typie.screen.more.stats

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.datetime.LocalDate

class StatsCalculatorTest {

  @Test
  fun calculateStreakData_countsCurrentLongestAndMonthlyDays() {
    val changes =
      listOf(
        StatsCharacterCountChange(date = LocalDate(2026, 3, 17), additions = 40, deletions = 0),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 19), additions = 80, deletions = -5),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 20), additions = 120, deletions = -8),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 21), additions = 160, deletions = -13),
      )

    val result =
      calculateStreakData(
        characterCountChanges = changes,
        totalCharacterCount = 1_000,
        today = LocalDate(2026, 3, 21),
      )

    assertEquals(
      ActivityData(
        currentStreak = 3,
        longestStreak = 3,
        thisMonthActiveDays = 4,
        totalActiveDays = 4,
        averageCharacterCountPerDay = 250,
      ),
      result,
    )
  }

  @Test
  fun calculateStreakData_startsFromYesterdayWhenTodayHasNoActivity() {
    val changes =
      listOf(
        StatsCharacterCountChange(date = LocalDate(2026, 3, 19), additions = 70, deletions = 0),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 20), additions = 95, deletions = 0),
      )

    val result =
      calculateStreakData(
        characterCountChanges = changes,
        totalCharacterCount = 500,
        today = LocalDate(2026, 3, 21),
      )

    assertEquals(2, result.currentStreak)
    assertEquals(2, result.longestStreak)
    assertEquals(2, result.thisMonthDays)
    assertEquals(2, result.totalDays)
    assertEquals(250, result.avgCharactersPerDay)
  }

  @Test
  fun calculateWeekdayPattern_groupsBySundayFirstAndRoundsAverage() {
    val changes =
      listOf(
        StatsCharacterCountChange(date = LocalDate(2026, 3, 22), additions = 120, deletions = 0),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 23), additions = 80, deletions = 0),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 23), additions = 20, deletions = 0),
        StatsCharacterCountChange(date = LocalDate(2026, 3, 24), additions = 0, deletions = -10),
      )

    val result = calculateWeekdayPattern(changes)

    assertEquals(
      listOf(
        WeekdayActivityData(
          dayIndex = 0,
          label = "일",
          totalAdditions = 120,
          averageAdditions = 120,
          activeDays = 1,
        ),
        WeekdayActivityData(
          dayIndex = 1,
          label = "월",
          totalAdditions = 100,
          averageAdditions = 50,
          activeDays = 2,
        ),
        WeekdayActivityData(
          dayIndex = 2,
          label = "화",
          totalAdditions = 0,
          averageAdditions = 0,
          activeDays = 0,
        ),
        WeekdayActivityData(
          dayIndex = 3,
          label = "수",
          totalAdditions = 0,
          averageAdditions = 0,
          activeDays = 0,
        ),
        WeekdayActivityData(
          dayIndex = 4,
          label = "목",
          totalAdditions = 0,
          averageAdditions = 0,
          activeDays = 0,
        ),
        WeekdayActivityData(
          dayIndex = 5,
          label = "금",
          totalAdditions = 0,
          averageAdditions = 0,
          activeDays = 0,
        ),
        WeekdayActivityData(
          dayIndex = 6,
          label = "토",
          totalAdditions = 0,
          averageAdditions = 0,
          activeDays = 0,
        ),
      ),
      result,
    )
  }

  @Test
  fun generateActivityChartDays_fillsMissingDatesAndNormalizesDeletions() {
    val result =
      generateActivityChartDays(
        characterCountChanges =
          listOf(
            StatsCharacterCountChange(
              date = LocalDate(2026, 3, 20),
              additions = 60,
              deletions = -20,
            ),
            StatsCharacterCountChange(date = LocalDate(2026, 3, 22), additions = 40, deletions = 0),
          ),
        endDate = LocalDate(2026, 3, 22),
        dayCount = 3,
      )

    assertEquals(
      listOf(
        StatsActivityDay(date = LocalDate(2026, 3, 20), additions = 60, deletions = 20),
        StatsActivityDay(date = LocalDate(2026, 3, 21), additions = 0, deletions = 0),
        StatsActivityDay(date = LocalDate(2026, 3, 22), additions = 40, deletions = 0),
      ),
      result,
    )
  }
}
