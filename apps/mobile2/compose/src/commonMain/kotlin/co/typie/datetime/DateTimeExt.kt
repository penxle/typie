@file:OptIn(ExperimentalTime::class)

package co.typie.datetime

import kotlinx.datetime.LocalDate
import kotlinx.datetime.LocalDateTime
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlin.time.ExperimentalTime
import kotlin.time.Instant

fun String.toInstantOrNull(): Instant? = Instant.parseOrNull(this)

fun Instant.toLocalDateTime(): LocalDateTime =
  toLocalDateTime(TimeZone.currentSystemDefault())

fun Instant.toLocalDate(): LocalDate =
  toLocalDateTime().date
