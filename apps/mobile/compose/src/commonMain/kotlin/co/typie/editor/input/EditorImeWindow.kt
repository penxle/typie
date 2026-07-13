package co.typie.editor.input

import co.typie.editor.ffi.Ime

internal fun Ime.trimmedTo(beforeLimit: Int, afterLimit: Int): Ime {
  val windowEnd = windowStart + text.codePointOffsetAtUtf16Index(text.length)
  val start = (selection.start - beforeLimit).coerceIn(windowStart, windowEnd)
  val end = (selection.end + afterLimit).coerceIn(start, windowEnd)
  if (start == windowStart && end == windowEnd) return this
  val startUtf16 = text.utf16IndexAtCodePointOffset(start - windowStart)
  val endUtf16 = text.utf16IndexAtCodePointOffset(end - windowStart)
  return copy(text = text.substring(startUtf16, endUtf16), windowStart = start)
}
