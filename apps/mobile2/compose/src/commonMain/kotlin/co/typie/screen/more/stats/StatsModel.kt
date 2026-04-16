package co.typie.screen.more.stats

data class ActivityData(
  val currentStreak: Int,
  val longestStreak: Int,
  val totalActiveDays: Int,
  val thisMonthActiveDays: Int,
  val averageCharacterCountPerDay: Int,
  val weekdayActivities: List<WeekdayActivityData>,
  val mostActiveWeekdayIndex: Int?,
)

data class WeekdayActivityData(
  val dayIndex: Int,
  val totalAdditions: Int,
  val averageAdditions: Int,
  val activeDays: Int,
)
