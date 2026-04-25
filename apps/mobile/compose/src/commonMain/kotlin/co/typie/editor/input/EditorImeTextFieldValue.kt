// cspell:ignore DBFF DFFF

package co.typie.editor.input

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import co.typie.editor.ffi.Ime

internal fun Ime.toTextFieldValue(): TextFieldValue =
  TextFieldValue(
    text = text,
    selection =
      resolveImeTextRange(
        text = text,
        windowStart = windowStart,
        start = selection.start,
        end = selection.end,
      ),
    composition =
      composing?.let {
        resolveImeTextRange(text = text, windowStart = windowStart, start = it.start, end = it.end)
      },
  )

private fun resolveImeTextRange(text: String, windowStart: Int, start: Int, end: Int): TextRange {
  val textStart = resolveImeTextIndex(text = text, windowStart = windowStart, offset = start)
  val textEnd = resolveImeTextIndex(text = text, windowStart = windowStart, offset = end)
  return TextRange(start = textStart, end = textEnd)
}

private fun resolveImeTextIndex(text: String, windowStart: Int, offset: Int): Int {
  val relativeOffset =
    (offset.toLong() - windowStart.toLong()).coerceIn(0, Int.MAX_VALUE.toLong()).toInt()
  return text.utf16IndexAtCodePointOffset(relativeOffset)
}

private fun String.utf16IndexAtCodePointOffset(offset: Int): Int {
  var utf16Index = 0
  var remaining = offset
  while (utf16Index < length && remaining > 0) {
    utf16Index += codePointUtf16LengthAt(utf16Index)
    remaining--
  }
  return utf16Index.coerceAtMost(length)
}

private fun String.codePointUtf16LengthAt(index: Int): Int =
  if (
    this[index] in '\uD800'..'\uDBFF' && index + 1 < length && this[index + 1] in '\uDC00'..'\uDFFF'
  ) {
    2
  } else {
    1
  }
