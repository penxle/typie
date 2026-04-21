package co.typie.screen.more.stats

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.datetime.toLocalDate
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.StatsScreen_GenerateActivityImage_Mutation
import co.typie.graphql.StatsScreen_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.builder.buildUserUsage
import co.typie.graphql.executeMutation
import co.typie.graphql.text
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import kotlin.time.Clock
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.daysUntil
import kotlinx.datetime.isoDayNumber
import kotlinx.datetime.minus

class StatsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      StatsScreen_Query()
    }

  val activity by derivedStateOf {
    val today = Clock.System.now().toLocalDate()

    val activeChanges = query.data.me.characterCountChanges.filter { it.additions > 0 }
    val activeDays = activeChanges.mapTo(mutableSetOf()) { it.date.toLocalDate() }

    val start = if (today in activeDays) today else today.minus(1, DateTimeUnit.DAY)
    val currentStreak =
      generateSequence(start) { it.minus(1, DateTimeUnit.DAY) }
        .takeWhile { it in activeDays }
        .count()

    val sorted = activeDays.sorted()
    val longestStreak =
      if (sorted.isEmpty()) {
        0
      } else {
        sorted
          .zipWithNext { a, b -> a.daysUntil(b) == 1 }
          .fold(1 to 1) { (longest, current), isConsecutive ->
            val next = if (isConsecutive) current + 1 else 1
            maxOf(longest, next) to next
          }
          .first
      }

    val thisMonthActiveDays = activeDays.count { it.year == today.year && it.month == today.month }
    val totalActiveDays = activeDays.size
    val averageCharacterCountPerDay =
      if (totalActiveDays > 0) {
        query.data.me.usage.totalCharacterCount / totalActiveDays
      } else {
        0
      }
    val byDayOfWeek = activeChanges.groupBy { it.date.toLocalDate().dayOfWeek.isoDayNumber % 7 }
    val weekdayActivities =
      (0..<7).map { dayIndex ->
        val changes = byDayOfWeek[dayIndex].orEmpty()
        val totalAdditions = changes.sumOf { it.additions }
        WeekdayActivityData(
          dayIndex = dayIndex,
          totalAdditions = totalAdditions,
          averageAdditions =
            if (changes.isNotEmpty()) {
              totalAdditions / changes.size
            } else {
              0
            },
          activeDays = changes.size,
        )
      }
    val mostActiveWeekdayIndex =
      weekdayActivities.maxBy { it.averageAdditions }.takeIf { it.averageAdditions > 0 }?.dayIndex

    ActivityData(
      currentStreak = currentStreak,
      longestStreak = longestStreak,
      thisMonthActiveDays = thisMonthActiveDays,
      totalActiveDays = totalActiveDays,
      averageCharacterCountPerDay = averageCharacterCountPerDay,
      mostActiveWeekdayIndex = mostActiveWeekdayIndex,
      weekdayActivities = weekdayActivities,
    )
  }

  suspend fun generateActivityImage(): Result<ByteArray, Nothing> = result {
    val response = Apollo.executeMutation(StatsScreen_GenerateActivityImage_Mutation())
    response.generateActivityImage
  }
}

private fun placeholderData() =
  StatsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser {
      name = text(3..6)
      documentCount = 0
      usage = buildUserUsage { totalCharacterCount = 0 }
      characterCountChanges = emptyList()
    }
  }
