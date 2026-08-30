package co.typie.screen.goal.user

import co.typie.domain.goal.toUserGoalDays
import co.typie.graphql.UserGoalScreen_Query
import co.typie.graphql.fragment.UserGoalFields_user
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class UserGoalMappingTest {
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

  private fun change(date: Instant, additions: Int) =
    UserGoalScreen_Query.CharacterCountChange(
      __typename = "CharacterCountChange",
      date = date,
      additions = additions,
    )

  @Test
  fun characterCountChangesUseKstCalendarDay() {
    val byDate =
      listOf(
          change(Instant.parse("2026-08-05T15:00:00Z"), 1200),
          change(Instant.parse("2026-08-05T14:59:59Z"), 900),
        )
        .toAdditionsByDate()

    assertEquals(mapOf(LocalDate(2026, 8, 6) to 1200L, LocalDate(2026, 8, 5) to 900L), byDate)
  }

  @Test
  fun emptyCharacterCountChangesMapToEmptyMap() {
    assertEquals(
      emptyMap(),
      emptyList<UserGoalScreen_Query.CharacterCountChange>().toAdditionsByDate(),
    )
  }

  @Test
  fun achievementsAreKeyedByKstDate() {
    val achievements =
      listOf(
          historyEntry(Instant.parse("2026-08-04T15:00:00Z"), achieved = true),
          historyEntry(Instant.parse("2026-08-05T15:00:00Z"), achieved = false),
        )
        .toUserGoalDays()
        .toAchievementsByDate()

    assertEquals(
      mapOf(LocalDate(2026, 8, 5) to true, LocalDate(2026, 8, 6) to false),
      achievements,
    )
  }
}
