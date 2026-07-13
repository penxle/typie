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
  val start = ime.projectWindowUtf16Index(newValue.selection.start)
  val end = ime.projectWindowUtf16Index(newValue.selection.end)
  return SelectionOnlyEditCommandProjection.Target(ImeRange(start = start, end = end))
}
