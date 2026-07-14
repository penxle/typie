package co.typie.editor.input

import co.typie.editor.ffi.Ime

// Extracted-text snapshot in the connection's absolute offset convention:
// startOffset is the flat window start and selection offsets are UTF-16
// indices within the window text, so startOffset + selection is the exact
// inverse of projectAbsoluteUtf16Offset (getSurroundingText reports the same
// shape).
internal data class ImeExtract(
  val text: String,
  val startOffset: Int,
  val selectionStart: Int,
  val selectionEnd: Int,
)

internal fun Ime.extract(): ImeExtract =
  ImeExtract(
    text = text,
    startOffset = windowStart,
    selectionStart = text.utf16IndexAtCodePointOffset(selection.start - windowStart),
    selectionEnd = text.utf16IndexAtCodePointOffset(selection.end - windowStart),
  )
