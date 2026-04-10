package co.typie.datetime

import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import kotlin.time.Instant
import kotlinx.datetime.TimeZone

actual fun Instant.format(pattern: String, timeZone: TimeZone): String {
  val javaInstant = java.time.Instant.ofEpochSecond(epochSeconds, nanosecondsOfSecond.toLong())
  val zoned = javaInstant.atZone(ZoneId.of(timeZone.id))
  return zoned.format(DateTimeFormatter.ofPattern(pattern))
}

actual fun Instant.formatLocale(
  dateStyle: DateTimeStyle,
  timeStyle: DateTimeStyle?,
  timeZone: TimeZone,
): String {
  val javaInstant = java.time.Instant.ofEpochSecond(epochSeconds, nanosecondsOfSecond.toLong())
  val zoned = javaInstant.atZone(ZoneId.of(timeZone.id))
  val formatter =
    if (timeStyle != null) {
      DateTimeFormatter.ofLocalizedDateTime(
        dateStyle.toJavaFormatStyle(),
        timeStyle.toJavaFormatStyle(),
      )
    } else {
      DateTimeFormatter.ofLocalizedDate(dateStyle.toJavaFormatStyle())
    }
  return zoned.format(formatter)
}

private fun DateTimeStyle.toJavaFormatStyle(): FormatStyle =
  when (this) {
    DateTimeStyle.SHORT -> FormatStyle.SHORT
    DateTimeStyle.MEDIUM -> FormatStyle.MEDIUM
    DateTimeStyle.LONG -> FormatStyle.LONG
    DateTimeStyle.FULL -> FormatStyle.FULL
  }
