package co.typie.datetime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.time.Instant
import kotlinx.datetime.LocalDate

class KstTest {
  @Test
  fun utcEveningIsNextKstDate() {
    val instant = Instant.parse("2026-08-04T15:30:00Z")
    assertEquals(LocalDate(2026, 8, 5), instant.toKstLocalDate())
  }

  @Test
  fun utcAfternoonIsSameKstDate() {
    val instant = Instant.parse("2026-08-05T05:00:00Z")
    assertEquals(LocalDate(2026, 8, 5), instant.toKstLocalDate())
  }

  @Test
  fun kstStartOfDayRoundTrips() {
    val date = LocalDate(2026, 8, 5)
    assertEquals(Instant.parse("2026-08-04T15:00:00Z"), date.atKstStartOfDay())
    assertEquals(date, date.atKstStartOfDay().toKstLocalDate())
  }
}
