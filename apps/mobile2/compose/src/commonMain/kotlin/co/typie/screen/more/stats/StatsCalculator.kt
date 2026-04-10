package co.typie.screen.more.stats

import kotlinx.datetime.LocalDate
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlin.math.roundToInt
import kotlin.time.Clock

val weekdayLabels = listOf("일", "월", "화", "수", "목", "금", "토")

data class StatsCharacterCountChange(
  val date: LocalDate,
  val additions: Int,
  val deletions: Int,
)

data class StreakData(
  val currentStreak: Int,
  val longestStreak: Int,
  val thisMonthDays: Int,
  val totalDays: Int,
  val avgCharactersPerDay: Int,
)

data class WeekdayData(
  val dayIndex: Int,
  val label: String,
  val totalAdditions: Int,
  val avgAdditions: Int,
  val count: Int,
)

data class StatsActivityDay(
  val date: LocalDate,
  val additions: Int,
  val deletions: Int,
) {
  val total: Int = additions + deletions
}

fun calculateStreakData(
  characterCountChanges: List<StatsCharacterCountChange>,
  totalCharacterCount: Int,
  today: LocalDate = Clock.System.now().toLocalDateTime(TimeZone.currentSystemDefault()).date,
): StreakData {
  val activeDates = characterCountChanges
    .asSequence()
    .filter { it.additions > 0 }
    .map { it.date.toEpochDays() }
    .toSet()

  var currentStreak = 0
  var checkDate = today.toEpochDays()

  if (checkDate !in activeDates) {
    checkDate -= 1
  }

  while (checkDate in activeDates) {
    currentStreak += 1
    checkDate -= 1
  }

  val sortedDates = activeDates.sorted()
  var longestStreak = 0
  var tempStreak = 0

  sortedDates.forEachIndexed { index, epochDay ->
    tempStreak = if (index == 0 || epochDay - sortedDates[index - 1] != 1L) 1 else tempStreak + 1
    if (tempStreak > longestStreak) {
      longestStreak = tempStreak
    }
  }

  val thisMonthDays = activeDates.count { epochDay ->
    val date = LocalDate.fromEpochDays(epochDay)
    date.year == today.year && date.month == today.month
  }

  val totalDays = activeDates.size
  val avgCharactersPerDay = if (totalDays > 0) {
    (totalCharacterCount.toDouble() / totalDays).roundToInt()
  } else {
    0
  }

  return StreakData(
    currentStreak = currentStreak,
    longestStreak = longestStreak,
    thisMonthDays = thisMonthDays,
    totalDays = totalDays,
    avgCharactersPerDay = avgCharactersPerDay,
  )
}

fun calculateWeekdayPattern(
  characterCountChanges: List<StatsCharacterCountChange>,
): List<WeekdayData> {
  val totals = MutableList(7) { 0 }
  val counts = MutableList(7) { 0 }

  characterCountChanges.forEach { change ->
    if (change.additions <= 0) {
      return@forEach
    }

    val dayIndex = (change.date.dayOfWeek.ordinal + 1) % 7
    totals[dayIndex] += change.additions
    counts[dayIndex] += 1
  }

  return weekdayLabels.mapIndexed { index, label ->
    WeekdayData(
      dayIndex = index,
      label = label,
      totalAdditions = totals[index],
      avgAdditions = if (counts[index] > 0) {
        (totals[index].toDouble() / counts[index]).roundToInt()
      } else {
        0
      },
      count = counts[index],
    )
  }
}

fun generateActivityChartDays(
  characterCountChanges: List<StatsCharacterCountChange>,
  endDate: LocalDate = Clock.System.now().toLocalDateTime(TimeZone.currentSystemDefault()).date,
  dayCount: Int = 90,
): List<StatsActivityDay> {
  if (dayCount <= 0) {
    return emptyList()
  }

  val startEpochDay = endDate.toEpochDays() - dayCount + 1
  val changesByDate = characterCountChanges.associateBy { it.date.toEpochDays() }

  return (startEpochDay..endDate.toEpochDays()).map { epochDay ->
    val change = changesByDate[epochDay]
    StatsActivityDay(
      date = LocalDate.fromEpochDays(epochDay),
      additions = change?.additions ?: 0,
      deletions = kotlin.math.abs(change?.deletions ?: 0),
    )
  }
}
