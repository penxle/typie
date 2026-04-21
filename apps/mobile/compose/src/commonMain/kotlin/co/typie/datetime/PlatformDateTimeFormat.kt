@file:OptIn(ExperimentalTime::class)

package co.typie.datetime

import kotlin.time.ExperimentalTime
import kotlin.time.Instant
import kotlinx.datetime.TimeZone

expect fun Instant.format(
  pattern: String,
  timeZone: TimeZone = TimeZone.currentSystemDefault(),
): String

expect fun Instant.formatLocale(
  dateStyle: DateTimeStyle = DateTimeStyle.MEDIUM,
  timeStyle: DateTimeStyle? = null,
  timeZone: TimeZone = TimeZone.currentSystemDefault(),
): String
