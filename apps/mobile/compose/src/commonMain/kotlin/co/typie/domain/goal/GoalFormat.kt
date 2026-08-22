package co.typie.domain.goal

import co.typie.ext.comma
import kotlinx.datetime.LocalDate

enum class DueStatusVariant {
  Full,
  Compact,
}

data class DueStatus(val label: String, val warning: Boolean)

fun dueStatus(
  current: Long,
  target: Long,
  dueDate: LocalDate,
  today: LocalDate,
  variant: DueStatusVariant,
): DueStatus? {
  val state = goalColorState(current, target)
  val duePassed = dueDate < today

  if (state != GoalColorState.Under && duePassed) {
    return null
  }

  val required = requiredToday(current, target, dueDate, today)
  val warning = state == GoalColorState.Under && duePassed

  val suffix =
    when {
      required <= 0 -> ""
      warning -> " · ${required.comma}자 남음"
      variant == DueStatusVariant.Full -> " · 오늘 ${required.comma}자 필요"
      else -> " · ${required.comma}자"
    }

  return DueStatus(label = "${dDayLabel(dueDate, today)}$suffix", warning = warning)
}
