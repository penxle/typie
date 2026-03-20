package co.typie.datetime

import kotlinx.datetime.TimeZone
import platform.Foundation.NSDate
import platform.Foundation.NSDateFormatter
import platform.Foundation.NSDateFormatterFullStyle
import platform.Foundation.NSDateFormatterLongStyle
import platform.Foundation.NSDateFormatterMediumStyle
import platform.Foundation.NSDateFormatterNoStyle
import platform.Foundation.NSDateFormatterShortStyle
import platform.Foundation.NSDateFormatterStyle
import platform.Foundation.NSTimeZone
import platform.Foundation.timeZoneWithName
import kotlin.time.Instant

private const val UNIX_TO_APPLE_EPOCH_OFFSET = 978307200.0

private fun Instant.toNSDate(): NSDate {
  val epoch = epochSeconds.toDouble() + nanosecondsOfSecond.toDouble() / 1_000_000_000.0
  return NSDate(timeIntervalSinceReferenceDate = epoch - UNIX_TO_APPLE_EPOCH_OFFSET)
}

actual fun Instant.format(pattern: String, timeZone: TimeZone): String {
  val formatter = NSDateFormatter().apply {
    dateFormat = pattern
    this.timeZone = NSTimeZone.timeZoneWithName(timeZone.id)!!
  }
  return formatter.stringFromDate(toNSDate())
}

actual fun Instant.formatLocale(
  dateStyle: DateTimeStyle,
  timeStyle: DateTimeStyle?,
  timeZone: TimeZone
): String {
  val formatter = NSDateFormatter().apply {
    this.dateStyle = dateStyle.toNSDateFormatterStyle()
    this.timeStyle = timeStyle?.toNSDateFormatterStyle() ?: NSDateFormatterNoStyle
    this.timeZone = NSTimeZone.timeZoneWithName(timeZone.id)!!
  }
  return formatter.stringFromDate(toNSDate())
}

private fun DateTimeStyle.toNSDateFormatterStyle(): NSDateFormatterStyle = when (this) {
  DateTimeStyle.SHORT -> NSDateFormatterShortStyle
  DateTimeStyle.MEDIUM -> NSDateFormatterMediumStyle
  DateTimeStyle.LONG -> NSDateFormatterLongStyle
  DateTimeStyle.FULL -> NSDateFormatterFullStyle
}
