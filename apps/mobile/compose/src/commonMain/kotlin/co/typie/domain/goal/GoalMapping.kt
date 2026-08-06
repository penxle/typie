package co.typie.domain.goal

import co.typie.datetime.toKstLocalDate
import co.typie.graphql.fragment.EntityGoalFields_goal
import co.typie.graphql.fragment.UserGoalFields_user

fun EntityGoalFields_goal.toEntityGoalData(): EntityGoalData =
  EntityGoalData(
    id = id,
    targetCharacterCount = targetCharacterCount.toLong(),
    dueDate = dueAt?.toKstLocalDate(),
    createdDate = createdAt.toKstLocalDate(),
  )

fun UserGoalFields_user.GoalHistory.toUserGoalDay(): UserGoalDay =
  UserGoalDay(
    date = date.toKstLocalDate(),
    targetCharacterCount = targetCharacterCount.toLong(),
    additions = additions.toLong(),
    achieved = achieved,
  )

fun List<UserGoalFields_user.GoalHistory>.toUserGoalDays(): List<UserGoalDay> {
  return map { it.toUserGoalDay() }.sortedBy { it.date }
}
