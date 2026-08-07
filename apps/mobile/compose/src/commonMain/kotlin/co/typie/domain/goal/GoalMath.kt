package co.typie.domain.goal

import co.typie.datetime.atKstStartOfDay
import kotlin.time.Duration
import kotlin.time.Instant
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.LocalDate
import kotlinx.datetime.daysUntil
import kotlinx.datetime.minus
import kotlinx.datetime.plus

const val GOAL_OVER_RATIO: Double = 1.1
const val GOAL_EXCESS_RATIO: Double = 1.25

enum class GoalColorState {
  Under,
  Achieved,
  Over,
  Excess,
}

data class UserGoalDay(
  val date: LocalDate,
  val targetCharacterCount: Long,
  val additions: Long,
  val achieved: Boolean,
)

data class TodayProgress(val additions: Long, val achieved: Boolean)

data class Streaks(val current: Int, val best: Int)

data class EntityGoalData(
  val id: String,
  val targetCharacterCount: Long,
  val dueDate: LocalDate?,
  val createdDate: LocalDate,
)

data class GoalSourceCandidate(
  val entityId: String,
  val goal: EntityGoalData?,
  val folderCharacterCount: Long?,
)

data class GoalSource(
  val goal: EntityGoalData,
  val current: Long,
  val isFolder: Boolean,
  val entityId: String,
)

fun goalColorState(current: Long, target: Long): GoalColorState =
  when {
    current < target -> GoalColorState.Under
    current <= target * GOAL_OVER_RATIO -> GoalColorState.Achieved
    current <= target * GOAL_EXCESS_RATIO -> GoalColorState.Over
    else -> GoalColorState.Excess
  }

fun requiredToday(current: Long, target: Long, dueDate: LocalDate, today: LocalDate): Long {
  val remaining = target - current
  if (remaining <= 0) {
    return 0
  }

  val daysLeft = maxOf(1, today.daysUntil(dueDate) + 1)
  return (remaining + daysLeft - 1) / daysLeft
}

fun timeFraction(createdDate: LocalDate, dueDate: LocalDate, now: Instant): Float {
  val start = createdDate.atKstStartOfDay()
  val end = dueDate.plus(1, DateTimeUnit.DAY).atKstStartOfDay()
  val total = end - start

  if (total <= Duration.ZERO) {
    return 1f
  }

  return ((now - start) / total).toFloat().coerceIn(0f, 1f)
}

fun dDayLabel(dueDate: LocalDate, today: LocalDate): String {
  val diff = today.daysUntil(dueDate)

  return when {
    diff == 0 -> "D-DAY"
    diff > 0 -> "D-$diff"
    else -> "D+${-diff}"
  }
}

fun todayProgress(history: List<UserGoalDay>, today: LocalDate): TodayProgress {
  val last = history.lastOrNull()
  if (last == null || last.date != today) {
    return TodayProgress(0, false)
  }

  return TodayProgress(last.additions, last.achieved)
}

fun streaks(history: List<UserGoalDay>, today: LocalDate): Streaks {
  var best = 0
  var run = 0
  var prevAchievedDay: LocalDate? = null
  for (day in history) {
    if (!day.achieved) {
      run = 0
      prevAchievedDay = null
      continue
    }

    run = if (prevAchievedDay?.daysUntil(day.date) == 1) run + 1 else 1
    best = maxOf(best, run)
    prevAchievedDay = day.date
  }

  val achievedByDay = history.associate { it.date to it.achieved }
  var cursor = if (achievedByDay[today] == true) today else today.minus(1, DateTimeUnit.DAY)
  var current = 0
  while (achievedByDay[cursor] == true) {
    current += 1
    cursor = cursor.minus(1, DateTimeUnit.DAY)
  }

  return Streaks(current, best)
}

fun pickGoalSource(
  entityId: String,
  ownGoal: EntityGoalData?,
  ownCurrent: Long,
  ancestors: List<GoalSourceCandidate>,
): GoalSource? {
  if (ownGoal != null) {
    return GoalSource(ownGoal, ownCurrent, isFolder = false, entityId = entityId)
  }

  val ancestor = ancestors.lastOrNull { it.goal != null } ?: return null
  val goal = ancestor.goal ?: return null
  val current = ancestor.folderCharacterCount ?: return null

  return GoalSource(goal, current, isFolder = true, entityId = ancestor.entityId)
}
