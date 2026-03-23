@file:OptIn(ExperimentalTime::class)

package co.typie.datetime

import kotlin.time.Clock
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.minutes
import kotlin.time.ExperimentalTime
import kotlin.time.Instant

enum class DateTimeStyle {
  SHORT,
  MEDIUM,
  LONG,
  FULL,
}

fun Instant.timeAgo(now: Instant = Clock.System.now()): String {
  val duration = now - this
  val isPast = duration.isPositive()
  val abs = if (isPast) duration else -duration

  val text = when {
    abs < 1.minutes -> return "방금"
    abs < 1.hours -> "${abs.inWholeMinutes}분"
    abs < 1.days -> "${abs.inWholeHours}시간"
    abs < 30.days -> "${abs.inWholeDays}일"
    abs < 365.days -> "${abs.inWholeDays / 30}개월"
    else -> "${abs.inWholeDays / 365}년"
  }

  return if (isPast) "$text 전" else "$text 후"
}
