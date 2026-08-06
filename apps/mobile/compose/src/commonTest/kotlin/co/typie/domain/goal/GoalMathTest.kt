package co.typie.domain.goal

import co.typie.datetime.atKstStartOfDay
import co.typie.datetime.toKstLocalDate
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.time.Duration.Companion.hours
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class GoalMathTest {
  private val today = LocalDate(2026, 8, 5)

  @Test
  fun colorStateUnderBelowTarget() {
    assertEquals(GoalColorState.Under, goalColorState(0, 1000))
    assertEquals(GoalColorState.Under, goalColorState(999, 1000))
    assertEquals(GoalColorState.Achieved, goalColorState(1000, 1000))
  }

  @Test
  fun colorStateBoundaries() {
    assertEquals(GoalColorState.Achieved, goalColorState(1100, 1000))
    assertEquals(GoalColorState.Over, goalColorState(1101, 1000))
    assertEquals(GoalColorState.Over, goalColorState(1250, 1000))
    assertEquals(GoalColorState.Excess, goalColorState(1251, 1000))
  }

  @Test
  fun colorStateRatioConstants() {
    assertEquals(1.1, GOAL_OVER_RATIO)
    assertEquals(1.25, GOAL_EXCESS_RATIO)
  }

  @Test
  fun requiredTodaySplitsEvenly() {
    assertEquals(3000L, requiredToday(1000, 10_000, LocalDate(2026, 8, 7), today))
  }

  @Test
  fun requiredTodayOnDueDateIsAllRemaining() {
    assertEquals(9000L, requiredToday(1000, 10_000, LocalDate(2026, 8, 5), today))
  }

  @Test
  fun requiredTodayAfterDueDateIsAllRemaining() {
    assertEquals(9000L, requiredToday(1000, 10_000, LocalDate(2026, 8, 1), today))
  }

  @Test
  fun requiredTodayIsZeroWhenReached() {
    assertEquals(0L, requiredToday(10_000, 10_000, LocalDate(2026, 8, 7), today))
  }

  @Test
  fun timeFractionMidSpan() {
    val created = LocalDate(2026, 8, 1)
    val due = LocalDate(2026, 8, 11)

    assertEquals(
      5f / 11f,
      timeFraction(created, due, LocalDate(2026, 8, 6).atKstStartOfDay()),
      1e-4f,
    )
    assertEquals(1f, timeFraction(created, due, LocalDate(2026, 9, 1).atKstStartOfDay()))
    assertEquals(0f, timeFraction(created, due, LocalDate(2026, 7, 1).atKstStartOfDay()))
  }

  @Test
  fun timeFractionFullOnlyAfterDueMidnight() {
    val created = LocalDate(2026, 8, 1)
    val due = LocalDate(2026, 8, 11)

    assertEquals(
      10f / 11f,
      timeFraction(created, due, LocalDate(2026, 8, 11).atKstStartOfDay()),
      1e-4f,
    )
    assertEquals(1f, timeFraction(created, due, LocalDate(2026, 8, 12).atKstStartOfDay()))
  }

  @Test
  fun timeFractionSameDayGoalFillsOverOneDay() {
    val date = LocalDate(2026, 8, 5)

    assertEquals(0f, timeFraction(date, date, date.atKstStartOfDay()))
    assertEquals(0.5f, timeFraction(date, date, date.atKstStartOfDay() + 12.hours), 1e-4f)
  }

  @Test
  fun timeFractionDuePassedAtCreation() {
    assertEquals(
      1f,
      timeFraction(
        LocalDate(2026, 8, 5),
        LocalDate(2026, 8, 4),
        LocalDate(2026, 8, 5).atKstStartOfDay(),
      ),
    )
  }

  @Test
  fun dDayLabels() {
    assertEquals("D-3", dDayLabel(LocalDate(2026, 8, 8), today))
    assertEquals("D-DAY", dDayLabel(LocalDate(2026, 8, 5), today))
    assertEquals("D+2", dDayLabel(LocalDate(2026, 8, 3), today))
  }

  @Test
  fun todayProgressUsesLastEntryOnToday() {
    val history =
      listOf(
        UserGoalDay(Instant.parse("2026-08-03T15:00:00Z").toKstLocalDate(), 1000, 100, false),
        UserGoalDay(Instant.parse("2026-08-05T05:00:00Z").toKstLocalDate(), 1000, 1200, true),
      )

    assertEquals(TodayProgress(additions = 1200, achieved = true), todayProgress(history, today))
  }

  @Test
  fun todayProgressIsZeroWhenLastEntryIsNotToday() {
    val history =
      listOf(UserGoalDay(Instant.parse("2026-08-04T05:00:00Z").toKstLocalDate(), 1000, 1200, true))

    assertEquals(TodayProgress(additions = 0, achieved = false), todayProgress(history, today))
  }

  @Test
  fun todayProgressIsZeroWhenHistoryIsEmpty() {
    assertEquals(TodayProgress(additions = 0, achieved = false), todayProgress(emptyList(), today))
  }

  @Test
  fun todayProgressFollowsServerAchieved() {
    val history =
      listOf(UserGoalDay(Instant.parse("2026-08-05T05:00:00Z").toKstLocalDate(), 1000, 1200, false))

    assertEquals(TodayProgress(additions = 1200, achieved = false), todayProgress(history, today))
  }

  @Test
  fun todayProgressTreatsKstBoundaryAsToday() {
    val history =
      listOf(UserGoalDay(Instant.parse("2026-08-04T15:30:00Z").toKstLocalDate(), 1000, 300, false))

    assertEquals(TodayProgress(additions = 300, achieved = false), todayProgress(history, today))
  }

  @Test
  fun pickGoalSourceUsesOwnGoalWithOwnCurrent() {
    val goal = goalData()
    val ancestors = listOf(GoalSourceCandidate("a1", goal, 500))

    assertEquals(
      GoalSource(goal = goal, current = 300, isFolder = false, entityId = "e1"),
      pickGoalSource("e1", goal, 300, ancestors),
    )
  }

  @Test
  fun pickGoalSourceFallsBackToNearestAncestorGoal() {
    val goal = goalData()
    val ancestors =
      listOf(GoalSourceCandidate("a1", goal, 500), GoalSourceCandidate("a2", null, 700))

    assertEquals(
      GoalSource(goal = goal, current = 500, isFolder = true, entityId = "a1"),
      pickGoalSource("e1", null, 300, ancestors),
    )
  }

  @Test
  fun pickGoalSourceReturnsNullWithoutAnyGoal() {
    val ancestors = listOf(GoalSourceCandidate("a1", null, 500))

    assertNull(pickGoalSource("e1", null, 300, ancestors))
  }

  @Test
  fun streaksIncludeTodayWhenAchieved() {
    val history =
      listOf(
        UserGoalDay(LocalDate(2026, 8, 3), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 4), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 5), 1000, 1200, true),
      )

    assertEquals(Streaks(current = 3, best = 3), streaks(history, today))
  }

  @Test
  fun streaksTodayMissDoesNotBreak() {
    val history =
      listOf(
        UserGoalDay(LocalDate(2026, 8, 3), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 4), 1000, 1500, true),
        UserGoalDay(LocalDate(2026, 8, 5), 1000, 300, false),
      )

    assertEquals(Streaks(current = 2, best = 2), streaks(history, today))
  }

  @Test
  fun streaksMissedDayBreaksRun() {
    val history =
      listOf(
        UserGoalDay(LocalDate(2026, 8, 1), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 2), 1000, 300, false),
        UserGoalDay(LocalDate(2026, 8, 3), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 4), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 5), 1000, 1200, true),
      )

    assertEquals(Streaks(current = 3, best = 3), streaks(history, today))
  }

  @Test
  fun streaksMissingRowBreaksRun() {
    val history =
      listOf(
        UserGoalDay(LocalDate(2026, 8, 1), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 2), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 4), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 5), 1000, 1200, true),
      )

    assertEquals(Streaks(current = 2, best = 2), streaks(history, today))
  }

  @Test
  fun streaksBestComesFromPastRun() {
    val history =
      listOf(
        UserGoalDay(LocalDate(2026, 7, 28), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 7, 29), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 7, 30), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 7, 31), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 1), 1000, 300, false),
        UserGoalDay(LocalDate(2026, 8, 4), 1000, 1200, true),
        UserGoalDay(LocalDate(2026, 8, 5), 1000, 1200, true),
      )

    assertEquals(Streaks(current = 2, best = 4), streaks(history, today))
  }

  @Test
  fun streaksEmptyHistoryIsZero() {
    assertEquals(Streaks(current = 0, best = 0), streaks(emptyList(), today))
  }

  private fun goalData() =
    EntityGoalData(
      id = "g1",
      targetCharacterCount = 1000,
      dueDate = LocalDate(2026, 8, 7),
      createdDate = LocalDate(2026, 8, 1),
    )
}
