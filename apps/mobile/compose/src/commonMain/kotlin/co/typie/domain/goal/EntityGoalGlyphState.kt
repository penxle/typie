package co.typie.domain.goal

import kotlin.time.Instant
import kotlinx.datetime.LocalDate

internal data class EntityGoalGlyphState(
  val colorState: GoalColorState,
  val pie: Float?,
  val pieWarning: Boolean,
)

internal fun entityGoalGlyphState(
  current: Long,
  target: Long,
  createdDate: LocalDate,
  dueDate: LocalDate?,
  today: LocalDate,
  now: Instant,
): EntityGoalGlyphState {
  val colorState = goalColorState(current, target)
  val under = colorState == GoalColorState.Under
  val duePassed = dueDate != null && dueDate < today

  return EntityGoalGlyphState(
    colorState = colorState,
    pie = if (under && dueDate != null) timeFraction(createdDate, dueDate, now) else null,
    pieWarning = duePassed && under,
  )
}
