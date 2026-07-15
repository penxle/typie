package co.typie.editor.input

import co.typie.editor.ffi.Ime

// Extracted-text snapshot in the window-relative world: the window text is
// presented as the whole document (startOffset 0 on the Android side), so
// selection offsets are UTF-16 indices within the window text and round-trip
// through projectWindowUtf16Index.
internal data class ImeExtract(
  val text: String,
  val selectionStart: Int,
  val selectionEnd: Int,
)

internal fun Ime.extract(): ImeExtract =
  ImeExtract(
    text = text,
    selectionStart = windowUtf16Offset(selection.start),
    selectionEnd = windowUtf16Offset(selection.end),
  )
