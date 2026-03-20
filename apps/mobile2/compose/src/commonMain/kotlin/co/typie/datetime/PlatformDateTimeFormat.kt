@file:OptIn(ExperimentalTime::class)

package co.typie.datetime

import kotlinx.datetime.TimeZone
import kotlin.time.ExperimentalTime
import kotlin.time.Instant

expect fun Instant.format(
  pattern: String,
  timeZone: TimeZone = TimeZone.currentSystemDefault(),
): String

expect fun Instant.formatLocale(
  dateStyle: DateTimeStyle = DateTimeStyle.MEDIUM,
  timeStyle: DateTimeStyle? = null,
  timeZone: TimeZone = TimeZone.currentSystemDefault(),
): String
