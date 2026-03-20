package co.typie.datetime

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.seconds
import kotlin.time.Instant

class DateTimeFormatTest {
  private val now = Instant.parse("2026-03-21T12:00:00Z")

  @Test
  fun timeAgo_justNow() {
    assertEquals("방금", (now - 30.seconds).timeAgo(now))
  }

  @Test
  fun timeAgo_minutes() {
    assertEquals("5분 전", (now - 5.minutes).timeAgo(now))
    assertEquals("59분 전", (now - 59.minutes).timeAgo(now))
  }

  @Test
  fun timeAgo_hours() {
    assertEquals("2시간 전", (now - 2.hours).timeAgo(now))
    assertEquals("23시간 전", (now - 23.hours).timeAgo(now))
  }

  @Test
  fun timeAgo_days() {
    assertEquals("3일 전", (now - 3.days).timeAgo(now))
    assertEquals("29일 전", (now - 29.days).timeAgo(now))
  }

  @Test
  fun timeAgo_months() {
    assertEquals("2개월 전", (now - 60.days).timeAgo(now))
  }

  @Test
  fun timeAgo_years() {
    assertEquals("1년 전", (now - 400.days).timeAgo(now))
  }

  @Test
  fun timeAgo_future() {
    assertEquals("5분 후", (now + 5.minutes).timeAgo(now))
  }

  @Test
  fun toInstantOrNull_valid() {
    assertNotNull("2026-03-21T14:30:00Z".toInstantOrNull())
  }

  @Test
  fun toInstantOrNull_invalid() {
    assertNull("not-a-date".toInstantOrNull())
  }
}
