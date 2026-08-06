package co.typie.domain.goal

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.datetime.LocalDate

class EntityGoalHistoryTableTest {
  private fun point(day: Int, characterCount: Long) =
    CharacterCountPoint(LocalDate(2026, 8, day), characterCount)

  @Test
  fun emptyHistoryProducesNoRows() {
    assertEquals(emptyList(), goalHistoryRows(emptyList()))
  }

  @Test
  fun singlePointKeepsItsDiffLessRow() {
    val rows = goalHistoryRows(listOf(point(1, 100)))

    assertEquals(listOf(GoalHistoryRow(LocalDate(2026, 8, 1), 100, null)), rows)
  }

  @Test
  fun rowsRunNewestFirstWithDiffAgainstOlderRow() {
    val rows = goalHistoryRows(listOf(point(1, 100), point(2, 250), point(3, 200)))

    assertEquals(
      listOf(
        GoalHistoryRow(LocalDate(2026, 8, 3), 200, -50),
        GoalHistoryRow(LocalDate(2026, 8, 2), 250, 150),
      ),
      rows,
    )
  }

  @Test
  fun oldestRowIsDroppedOnceHistoryHasMoreThanOnePoint() {
    val rows = goalHistoryRows(listOf(point(1, 100), point(2, 250)))

    assertEquals(listOf(GoalHistoryRow(LocalDate(2026, 8, 2), 250, 150)), rows)
  }

  @Test
  fun windowKeepsLatestFifteenPointsBeforeDroppingTheDiffLessRow() {
    val history = (1..20).map { point(it, it * 100L) }
    val rows = goalHistoryRows(history)

    assertEquals(14, rows.size)
    assertEquals(LocalDate(2026, 8, 20), rows.first().date)
    assertEquals(LocalDate(2026, 8, 7), rows.last().date)
    assertTrue(rows.all { it.diff == 100L })
  }

  @Test
  fun diffLabelSignsEveryNonNullValue() {
    assertEquals("—", goalHistoryDiffLabel(null))
    assertEquals("+1,200", goalHistoryDiffLabel(1200))
    assertEquals("-300", goalHistoryDiffLabel(-300))
    assertEquals("+0", goalHistoryDiffLabel(0))
  }
}
