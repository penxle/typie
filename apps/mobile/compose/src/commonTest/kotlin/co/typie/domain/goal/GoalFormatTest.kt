package co.typie.domain.goal

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.datetime.LocalDate

class GoalFormatTest {
  private val today = LocalDate(2026, 8, 5)

  @Test
  fun dueStatusUnderBeforeDueDate() {
    val due = LocalDate(2026, 8, 7)

    assertEquals(
      DueStatus(label = "D-2 · 오늘 3,000자 필요", warning = false),
      dueStatus(1000, 10_000, due, today, DueStatusVariant.Full),
    )
    assertEquals(
      DueStatus(label = "D-2 · 3,000자", warning = false),
      dueStatus(1000, 10_000, due, today, DueStatusVariant.Compact),
    )
  }

  @Test
  fun dueStatusUnderOnDueDate() {
    val due = LocalDate(2026, 8, 5)

    assertEquals(
      DueStatus(label = "D-DAY · 오늘 9,000자 필요", warning = false),
      dueStatus(1000, 10_000, due, today, DueStatusVariant.Full),
    )
    assertEquals(
      DueStatus(label = "D-DAY · 9,000자", warning = false),
      dueStatus(1000, 10_000, due, today, DueStatusVariant.Compact),
    )
  }

  @Test
  fun dueStatusUnderAfterDueDateWarns() {
    val due = LocalDate(2026, 8, 3)

    assertEquals(
      DueStatus(label = "D+2 · 9,000자 남음", warning = true),
      dueStatus(1000, 10_000, due, today, DueStatusVariant.Full),
    )
    assertEquals(
      DueStatus(label = "D+2 · 9,000자 남음", warning = true),
      dueStatus(1000, 10_000, due, today, DueStatusVariant.Compact),
    )
  }

  @Test
  fun dueStatusAchievedBeforeDueDateShowsDDayOnly() {
    val due = LocalDate(2026, 8, 7)

    assertEquals(
      DueStatus(label = "D-2", warning = false),
      dueStatus(10_000, 10_000, due, today, DueStatusVariant.Full),
    )
    assertEquals(
      DueStatus(label = "D-2", warning = false),
      dueStatus(10_000, 10_000, due, today, DueStatusVariant.Compact),
    )
  }

  @Test
  fun dueStatusAchievedOnDueDateShowsDDayOnly() {
    val due = LocalDate(2026, 8, 5)

    assertEquals(
      DueStatus(label = "D-DAY", warning = false),
      dueStatus(10_000, 10_000, due, today, DueStatusVariant.Full),
    )
    assertEquals(
      DueStatus(label = "D-DAY", warning = false),
      dueStatus(10_000, 10_000, due, today, DueStatusVariant.Compact),
    )
  }

  @Test
  fun dueStatusAchievedAfterDueDateIsOmitted() {
    val due = LocalDate(2026, 8, 3)

    assertNull(dueStatus(10_000, 10_000, due, today, DueStatusVariant.Full))
    assertNull(dueStatus(10_000, 10_000, due, today, DueStatusVariant.Compact))
  }
}
