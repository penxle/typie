package co.typie.screen.goal.entity

import co.typie.datetime.atKstStartOfDay
import co.typie.domain.goal.GoalColorState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class EntityGoalHeroStateTest {
  private val today = LocalDate(2026, 8, 6)
  private val now = today.atKstStartOfDay()
  private val created = LocalDate(2026, 8, 1)

  private fun state(
    current: Long,
    target: Long = 10_000L,
    dueDate: LocalDate? = null,
    today: LocalDate = this.today,
    now: Instant = this.now,
  ) = entityGoalHeroState(current, target, created, dueDate, today, now)

  @Test
  fun percentFloorsTowardZero() {
    assertEquals(0, state(current = 99).percent)
    assertEquals(9, state(current = 999).percent)
    assertEquals(99, state(current = 9999).percent)
    assertEquals(100, state(current = 10_000).percent)
    assertEquals(150, state(current = 15_000).percent)
  }

  @Test
  fun overdueRequiresDueDateStrictlyBeforeToday() {
    assertEquals(false, state(current = 100, dueDate = null).overdue)
    assertEquals(false, state(current = 100, dueDate = today).overdue)
    assertEquals(false, state(current = 100, dueDate = LocalDate(2026, 8, 7)).overdue)
    assertEquals(true, state(current = 100, dueDate = LocalDate(2026, 8, 5)).overdue)
  }

  @Test
  fun overdueUnderOnlyWhenBelowTarget() {
    assertEquals(true, state(current = 100, dueDate = LocalDate(2026, 8, 5)).overdueUnder)
    assertEquals(false, state(current = 10_000, dueDate = LocalDate(2026, 8, 5)).overdueUnder)
  }

  @Test
  fun pieOnlyWhenDueDatePresentAndUnder() {
    assertNull(state(current = 100, dueDate = null).pie)
    assertNull(state(current = 10_000, dueDate = LocalDate(2026, 8, 10)).pie)

    val pie = state(current = 100, dueDate = LocalDate(2026, 8, 10)).pie
    assertTrue(pie != null && pie > 0f && pie < 1f, "pie should be a partial fraction, was $pie")
  }

  @Test
  fun requiredIsZeroWithoutDueDate() {
    assertEquals(0L, state(current = 100, dueDate = null).required)
  }

  @Test
  fun requiredSplitsRemainderAcrossRemainingDays() {
    assertEquals(3300L, state(current = 100, dueDate = LocalDate(2026, 8, 8)).required)
    assertEquals(0L, state(current = 10_000, dueDate = LocalDate(2026, 8, 8)).required)
  }

  @Test
  fun dueChipHiddenWithoutDueDate() {
    assertEquals(false, state(current = 100, dueDate = null).dueChipVisible)
  }

  @Test
  fun dueChipVisibleWhenUnderEvenAfterDueDate() {
    assertEquals(true, state(current = 100, dueDate = LocalDate(2026, 8, 5)).dueChipVisible)
  }

  @Test
  fun dueChipHiddenWhenReachedAndDuePassed() {
    val reached = state(current = 10_000, dueDate = LocalDate(2026, 8, 5))

    assertEquals(GoalColorState.Achieved, reached.colorState)
    assertEquals(false, reached.dueChipVisible)
  }

  @Test
  fun dueChipVisibleWhenReachedBeforeDueDate() {
    assertEquals(true, state(current = 10_000, dueDate = LocalDate(2026, 8, 10)).dueChipVisible)
  }
}
