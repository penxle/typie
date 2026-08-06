package co.typie.domain.goal

import co.typie.graphql.fragment.EntityGoalFields_goal
import co.typie.graphql.fragment.UserGoalFields_user
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class GoalMappingTest {
  private fun goal(
    dueAt: Instant?,
    createdAt: Instant,
    id: String = "goal-1",
    targetCharacterCount: Int = 10_000,
  ) =
    EntityGoalFields_goal(
      __typename = "EntityGoal",
      id = id,
      targetCharacterCount = targetCharacterCount,
      dueAt = dueAt,
      createdAt = createdAt,
    )

  private fun historyEntry(
    date: Instant,
    targetCharacterCount: Int = 1000,
    additions: Int = 0,
    achieved: Boolean = false,
  ) =
    UserGoalFields_user.GoalHistory(
      __typename = "UserGoalHistory",
      date = date,
      targetCharacterCount = targetCharacterCount,
      additions = additions,
      achieved = achieved,
    )

  @Test
  fun entityGoalDatesUseKstCalendarDay() {
    val data =
      goal(
          dueAt = Instant.parse("2026-08-05T15:00:00Z"),
          createdAt = Instant.parse("2026-07-31T15:00:00Z"),
        )
        .toEntityGoalData()

    assertEquals("goal-1", data.id)
    assertEquals(10_000L, data.targetCharacterCount)
    assertEquals(LocalDate(2026, 8, 6), data.dueDate)
    assertEquals(LocalDate(2026, 8, 1), data.createdDate)
  }

  @Test
  fun entityGoalDatesDoNotRollOverBeforeKstMidnight() {
    val data =
      goal(
          dueAt = Instant.parse("2026-08-05T14:59:59Z"),
          createdAt = Instant.parse("2026-07-31T14:59:59Z"),
        )
        .toEntityGoalData()

    assertEquals(LocalDate(2026, 8, 5), data.dueDate)
    assertEquals(LocalDate(2026, 7, 31), data.createdDate)
  }

  @Test
  fun entityGoalWithoutDueAtHasNullDueDate() {
    val data =
      goal(dueAt = null, createdAt = Instant.parse("2026-08-01T00:00:00Z")).toEntityGoalData()

    assertNull(data.dueDate)
    assertEquals(LocalDate(2026, 8, 1), data.createdDate)
  }

  @Test
  fun goalHistoryUsesKstCalendarDay() {
    val days =
      listOf(
          historyEntry(
            Instant.parse("2026-08-05T15:00:00Z"),
            targetCharacterCount = 1000,
            additions = 1200,
            achieved = true,
          )
        )
        .toUserGoalDays()

    assertEquals(listOf(UserGoalDay(LocalDate(2026, 8, 6), 1000L, 1200L, true)), days)
  }

  @Test
  fun goalHistoryDoesNotRollOverBeforeKstMidnight() {
    val days = listOf(historyEntry(Instant.parse("2026-08-05T14:59:59Z"))).toUserGoalDays()

    assertEquals(listOf(LocalDate(2026, 8, 5)), days.map { it.date })
  }

  @Test
  fun goalHistoryIsSortedAscending() {
    val days =
      listOf(
          historyEntry(Instant.parse("2026-08-04T15:00:00Z"), additions = 300),
          historyEntry(Instant.parse("2026-08-02T15:00:00Z"), additions = 100),
          historyEntry(Instant.parse("2026-08-03T15:00:00Z"), additions = 200),
        )
        .toUserGoalDays()

    assertEquals(
      listOf(LocalDate(2026, 8, 3), LocalDate(2026, 8, 4), LocalDate(2026, 8, 5)),
      days.map { it.date },
    )
    assertEquals(listOf(100L, 200L, 300L), days.map { it.additions })
  }

  @Test
  fun emptyGoalHistoryMapsToEmptyList() {
    assertEquals(emptyList(), emptyList<UserGoalFields_user.GoalHistory>().toUserGoalDays())
  }

  @Test
  fun todayProgressReadsKstMappedHistory() {
    val days =
      listOf(
          historyEntry(Instant.parse("2026-08-04T15:00:00Z"), additions = 900, achieved = false),
          historyEntry(Instant.parse("2026-08-05T15:00:00Z"), additions = 1200, achieved = true),
        )
        .toUserGoalDays()

    assertEquals(1200L, todayProgress(days, LocalDate(2026, 8, 6)).additions)
    assertEquals(true, todayProgress(days, LocalDate(2026, 8, 6)).achieved)
    assertEquals(0L, todayProgress(days, LocalDate(2026, 8, 5)).additions)
  }

  @Test
  fun streakCountsKstMappedAchievedRun() {
    val days =
      listOf(
          historyEntry(Instant.parse("2026-08-03T15:00:00Z"), achieved = true),
          historyEntry(Instant.parse("2026-08-04T15:00:00Z"), achieved = true),
          historyEntry(Instant.parse("2026-08-05T15:00:00Z"), achieved = true),
        )
        .toUserGoalDays()

    assertEquals(3, streaks(days, LocalDate(2026, 8, 6)).current)
  }
}
