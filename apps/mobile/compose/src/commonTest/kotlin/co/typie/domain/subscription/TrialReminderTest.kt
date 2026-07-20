package co.typie.domain.subscription

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.datetime.LocalDate

class TrialReminderTest {
  private val today = LocalDate(2026, 7, 20)

  @Test
  fun `not shown when more than 3 days left`() {
    assertFalse(shouldShowTrialReminder(daysLeft = 4, today = today, lastShownDate = null))
  }

  @Test
  fun `shown at 3 days left on first visit`() {
    assertTrue(shouldShowTrialReminder(daysLeft = 3, today = today, lastShownDate = null))
  }

  @Test
  fun `shown on the last day`() {
    assertTrue(shouldShowTrialReminder(daysLeft = 0, today = today, lastShownDate = null))
  }

  @Test
  fun `not shown twice on the same day`() {
    assertFalse(shouldShowTrialReminder(daysLeft = 2, today = today, lastShownDate = "2026-07-20"))
  }

  @Test
  fun `shown again on the next day`() {
    assertTrue(shouldShowTrialReminder(daysLeft = 2, today = today, lastShownDate = "2026-07-19"))
  }
}
