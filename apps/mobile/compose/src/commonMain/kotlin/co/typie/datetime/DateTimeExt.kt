@file:OptIn(ExperimentalTime::class)

package co.typie.datetime

import kotlin.time.ExperimentalTime
import kotlin.time.Instant
import kotlinx.datetime.LocalDate
import kotlinx.datetime.LocalDateTime
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime

fun String.toInstantOrNull(): Instant? = Instant.parseOrNull(this)

fun Instant.toLocalDateTime(): LocalDateTime = toLocalDateTime(TimeZone.currentSystemDefault())

fun Instant.toLocalDate(): LocalDate = toLocalDateTime().date
