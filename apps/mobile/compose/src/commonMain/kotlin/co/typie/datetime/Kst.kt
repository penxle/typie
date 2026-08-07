@file:OptIn(ExperimentalTime::class)

package co.typie.datetime

import kotlin.time.Clock
import kotlin.time.ExperimentalTime
import kotlin.time.Instant
import kotlinx.datetime.LocalDate
import kotlinx.datetime.TimeZone
import kotlinx.datetime.atStartOfDayIn
import kotlinx.datetime.toLocalDateTime

val KstTimeZone = TimeZone.of("Asia/Seoul")

fun Instant.toKstLocalDate(): LocalDate = toLocalDateTime(KstTimeZone).date

fun kstToday(clock: Clock = Clock.System): LocalDate = clock.now().toKstLocalDate()

fun LocalDate.atKstStartOfDay(): Instant = atStartOfDayIn(KstTimeZone)
