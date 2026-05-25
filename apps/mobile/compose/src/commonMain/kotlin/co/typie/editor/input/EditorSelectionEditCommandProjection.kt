// cspell:ignore DBFF DFFF

package co.typie.editor.input

import androidx.compose.ui.text.input.EditCommand
import androidx.compose.ui.text.input.EditProcessor
import androidx.compose.ui.text.input.MoveCursorCommand
import androidx.compose.ui.text.input.SetSelectionCommand
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange

internal sealed interface SelectionOnlyEditCommandProjection {
  data object MissingIme : SelectionOnlyEditCommandProjection

  data class Target(val range: ImeRange) : SelectionOnlyEditCommandProjection
}

internal fun List<EditCommand>.projectSelectionOnlyCommand(
  ime: Ime?
): SelectionOnlyEditCommandProjection? {
  if (isEmpty() || any { it !is SetSelectionCommand && it !is MoveCursorCommand }) return null
  if (ime == null) return SelectionOnlyEditCommandProjection.MissingIme

  val processor = EditProcessor()
  processor.reset(ime.toTextFieldValue(), null)
  val newValue = processor.apply(this)
  val start = ime.windowStart + ime.text.codePointOffsetAtUtf16Index(newValue.selection.start)
  val end = ime.windowStart + ime.text.codePointOffsetAtUtf16Index(newValue.selection.end)
  return SelectionOnlyEditCommandProjection.Target(ImeRange(start = start, end = end))
}

private fun String.codePointOffsetAtUtf16Index(index: Int): Int {
  var utf16Index = 0
  var codePointOffset = 0
  val target = index.coerceIn(0, length)
  while (utf16Index < target) {
    utf16Index += if (isHighSurrogateAt(utf16Index)) 2 else 1
    codePointOffset += 1
  }
  return codePointOffset
}

private fun String.isHighSurrogateAt(index: Int): Boolean =
  this[index] in '\uD800'..'\uDBFF' && index + 1 < length && this[index + 1] in '\uDC00'..'\uDFFF'
