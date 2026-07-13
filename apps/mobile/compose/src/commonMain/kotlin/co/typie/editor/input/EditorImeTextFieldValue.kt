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
