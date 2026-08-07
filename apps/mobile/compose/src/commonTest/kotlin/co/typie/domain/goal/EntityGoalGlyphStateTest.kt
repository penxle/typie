package co.typie.domain.goal

import co.typie.datetime.atKstStartOfDay
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class EntityGoalGlyphStateTest {
  private val today = LocalDate(2026, 8, 6)
  private val now = today.atKstStartOfDay()
  private val created = LocalDate(2026, 8, 1)

  private fun glyph(
    current: Long,
    target: Long = 10_000L,
    dueDate: LocalDate? = null,
    today: LocalDate = this.today,
    now: Instant = this.now,
  ) = entityGoalGlyphState(current, target, created, dueDate, today, now)

  @Test
  fun colorStateFollowsGoalColorState() {
    assertEquals(GoalColorState.Under, glyph(current = 9999).colorState)
    assertEquals(GoalColorState.Achieved, glyph(current = 10_000).colorState)
    assertEquals(GoalColorState.Over, glyph(current = 12_000).colorState)
    assertEquals(GoalColorState.Excess, glyph(current = 13_000).colorState)
  }

  @Test
  fun pieOnlyWhenDueDatePresentAndUnder() {
    assertNull(glyph(current = 100, dueDate = null).pie)
    assertNull(glyph(current = 10_000, dueDate = LocalDate(2026, 8, 10)).pie)

    val pie = glyph(current = 100, dueDate = LocalDate(2026, 8, 10)).pie
    assertTrue(pie != null && pie > 0f && pie < 1f, "pie should be a partial fraction, was $pie")
  }

  @Test
  fun pieWarningRequiresDuePassedAndUnder() {
    assertEquals(false, glyph(current = 100, dueDate = null).pieWarning)
    assertEquals(false, glyph(current = 100, dueDate = today).pieWarning)
    assertEquals(false, glyph(current = 100, dueDate = LocalDate(2026, 8, 7)).pieWarning)
    assertEquals(true, glyph(current = 100, dueDate = LocalDate(2026, 8, 5)).pieWarning)
    assertEquals(false, glyph(current = 10_000, dueDate = LocalDate(2026, 8, 5)).pieWarning)
  }

  @Test
  fun pieSurvivesAfterDuePassedWhileUnder() {
    val state = glyph(current = 100, dueDate = LocalDate(2026, 8, 5))
    assertEquals(1f, state.pie)
    assertEquals(true, state.pieWarning)
  }
}
