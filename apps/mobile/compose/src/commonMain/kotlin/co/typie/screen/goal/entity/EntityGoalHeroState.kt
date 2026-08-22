package co.typie.screen.goal.entity

import co.typie.domain.goal.GoalColorState
import co.typie.domain.goal.goalColorState
import co.typie.domain.goal.requiredToday
import co.typie.domain.goal.timeFraction
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

internal data class EntityGoalHeroState(
  val colorState: GoalColorState,
  val percent: Int,
  val overdue: Boolean,
  val overdueUnder: Boolean,
  val pie: Float?,
  val required: Long,
  val dueChipVisible: Boolean,
)

internal fun entityGoalHeroState(
  current: Long,
  target: Long,
  createdDate: LocalDate,
  dueDate: LocalDate?,
  today: LocalDate,
  now: Instant,
): EntityGoalHeroState {
  val colorState = goalColorState(current, target)
  val overdue = dueDate != null && dueDate < today
  val overdueUnder = overdue && colorState == GoalColorState.Under

  return EntityGoalHeroState(
    colorState = colorState,
    percent = (current.toDouble() / target * PERCENT_SCALE).toInt(),
    overdue = overdue,
    overdueUnder = overdueUnder,
    pie =
      if (dueDate != null && colorState == GoalColorState.Under) {
        timeFraction(createdDate, dueDate, now)
      } else {
        null
      },
    required = if (dueDate != null) requiredToday(current, target, dueDate, today) else 0L,
    dueChipVisible = dueDate != null && (colorState == GoalColorState.Under || !overdue),
  )
}

private const val PERCENT_SCALE = 100
