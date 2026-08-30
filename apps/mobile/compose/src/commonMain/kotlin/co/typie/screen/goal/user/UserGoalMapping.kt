package co.typie.screen.goal.user

import co.typie.datetime.toKstLocalDate
import co.typie.domain.goal.UserGoalDay
import co.typie.graphql.UserGoalScreen_Query
import kotlinx.datetime.LocalDate

internal fun List<UserGoalScreen_Query.CharacterCountChange>.toAdditionsByDate():
  Map<LocalDate, Long> = associate { it.date.toKstLocalDate() to it.additions.toLong() }

internal fun List<UserGoalDay>.toAchievementsByDate(): Map<LocalDate, Boolean> = associate {
  it.date to it.achieved
}
