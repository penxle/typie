package co.typie.domain.goal

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.datetime.LocalDate

class UserGoalRowsTest {
  @Test
  fun dotDaysCovers112EndingToday() {
    val today = LocalDate(2026, 8, 5)
    val days = dotDays(emptyList(), today)
    assertEquals(112, days.size)
    assertEquals(today, days.last().date)
    assertEquals(LocalDate(2026, 4, 16), days.first().date)
    assertNull(days.last().achieved)
  }

  @Test
  fun dailyRowsJoinJudgments() {
    val today = LocalDate(2026, 8, 5)
    val rows =
      dailyAdditionRows(
        changes = mapOf(LocalDate(2026, 8, 5) to 1200L),
        judgments = mapOf(LocalDate(2026, 8, 5) to true, LocalDate(2026, 8, 4) to false),
        today = today,
        days = 30,
      )
    assertEquals(30, rows.size)
    assertEquals(DailyAdditionRow(today, 1200, true), rows.first())
    assertEquals(DailyAdditionRow(LocalDate(2026, 8, 4), 0, false), rows[1])
    assertNull(rows[2].achieved)
  }

  @Test
  fun dotDaysJoinHistoryRowsAndLeaveGapsWithoutGoal() {
    val today = LocalDate(2026, 8, 5)
    val days =
      dotDays(
        history =
          listOf(
            goalDay(LocalDate(2026, 8, 3), additions = 0, achieved = false),
            goalDay(LocalDate(2026, 8, 5), additions = 1200, achieved = true),
          ),
        today = today,
      )

    val byDate = days.associateBy { it.date }
    assertEquals(DotDay(LocalDate(2026, 8, 5), 1200, true), byDate[LocalDate(2026, 8, 5)])
    assertEquals(DotDay(LocalDate(2026, 8, 3), 0, false), byDate[LocalDate(2026, 8, 3)])
    assertEquals(DotDay(LocalDate(2026, 8, 4), null, null), byDate[LocalDate(2026, 8, 4)])
  }

  @Test
  fun dotDaysDropRowsOutsideTheWindow() {
    val today = LocalDate(2026, 8, 5)
    val days =
      dotDays(
        history =
          listOf(
            goalDay(LocalDate(2026, 4, 15), additions = 500, achieved = true),
            goalDay(LocalDate(2026, 4, 16), additions = 700, achieved = true),
          ),
        today = today,
      )

    assertEquals(LocalDate(2026, 4, 16), days.first().date)
    assertEquals(700L, days.first().additions)
    assertEquals(1, days.count { it.additions != null })
  }

  @Test
  fun dailyAdditionRowsWalkBackwardFromToday() {
    val today = LocalDate(2026, 8, 5)
    val rows =
      dailyAdditionRows(changes = emptyMap(), judgments = emptyMap(), today = today, days = 5)

    assertEquals(5, rows.size)
    assertEquals(
      listOf(
        LocalDate(2026, 8, 5),
        LocalDate(2026, 8, 4),
        LocalDate(2026, 8, 3),
        LocalDate(2026, 8, 2),
        LocalDate(2026, 8, 1),
      ),
      rows.map { it.date },
    )
    assertEquals(listOf(0L, 0L, 0L, 0L, 0L), rows.map { it.additions })
  }

  @Test
  fun dailyAdditionRowsKeepJudgmentsWithoutChanges() {
    val today = LocalDate(2026, 8, 5)
    val rows =
      dailyAdditionRows(
        changes = mapOf(LocalDate(2026, 8, 4) to 300L),
        judgments = mapOf(LocalDate(2026, 8, 5) to false),
        today = today,
        days = 2,
      )

    assertEquals(DailyAdditionRow(LocalDate(2026, 8, 5), 0, false), rows[0])
    assertEquals(DailyAdditionRow(LocalDate(2026, 8, 4), 300, null), rows[1])
  }

  @Test
  fun dotDayMessageCoversFourStates() {
    val date = LocalDate(2026, 8, 5)

    assertEquals("8월 5일 수 · 1,200자 · 달성", dotDayMessage(DotDay(date, 1200, true)))
    assertEquals("8월 5일 수 · 300자 · 일부 달성", dotDayMessage(DotDay(date, 300, false)))
    assertEquals("8월 5일 수 · 0자 · 미달성", dotDayMessage(DotDay(date, 0, false)))
    assertEquals("8월 5일 수 · 목표 없음", dotDayMessage(DotDay(date, null, null)))
  }

  @Test
  fun dotDayMessageUsesShortWeekdayNames() {
    assertEquals("8월 2일 일 · 목표 없음", dotDayMessage(DotDay(LocalDate(2026, 8, 2), null, null)))
    assertEquals("8월 8일 토 · 목표 없음", dotDayMessage(DotDay(LocalDate(2026, 8, 8), null, null)))
  }

  @Test
  fun barYMaxTakesLargestOfFloorTargetAndRows() {
    val rows =
      listOf(
        DailyAdditionRow(LocalDate(2026, 8, 5), 3000, true),
        DailyAdditionRow(LocalDate(2026, 8, 4), 100, false),
      )

    assertEquals(1.05, userGoalBarYMax(emptyList(), target = null), 1e-9)
    assertEquals(1050.0, userGoalBarYMax(emptyList(), target = 1000), 1e-9)
    assertEquals(3150.0, userGoalBarYMax(rows, target = 1000), 1e-9)
    assertEquals(3150.0, userGoalBarYMax(rows, target = null), 1e-9)
  }

  @Test
  fun achievementLabelsFollowJudgment() {
    assertEquals("달성", goalAchievementLabel(true))
    assertEquals("미달성", goalAchievementLabel(false))
    assertEquals("—", goalAchievementLabel(null))
  }

  private fun goalDay(date: LocalDate, additions: Long, achieved: Boolean) =
    UserGoalDay(
      date = date,
      targetCharacterCount = 1000,
      additions = additions,
      achieved = achieved,
    )
}
