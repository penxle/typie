// cspell:ignore DBFF DFFF

package co.typie.editor.input

import co.typie.editor.ffi.Ime

// The Android boundary presents the window as the whole document: every
// coordinate exposed to or accepted from the keyboard is a UTF-16 index
// within the window text. Keyboards are only ever validated against stock
// editors that expose the full text, so a window-relative world (where the
// exposed prefix length, the reported selection, and extracted-text offsets
// all agree) is the only coordinate dialect they all speak.
internal fun Ime.projectWindowUtf16Index(index: Int): Int =
  windowStart + text.codePointOffsetAtUtf16Index(index)

internal fun Ime.windowUtf16Offset(flatOffset: Int): Int =
  text.utf16IndexAtCodePointOffset(flatOffset - windowStart)

internal fun String.codePointOffsetAtUtf16Index(index: Int): Int {
  var utf16Index = 0
  var codePointOffset = 0
  val target = index.coerceIn(0, length)
  while (utf16Index < target) {
    utf16Index += if (isHighSurrogateAt(utf16Index)) 2 else 1
    codePointOffset += 1
  }
  return codePointOffset
}

internal fun String.utf16IndexAtCodePointOffset(offset: Int): Int {
  var utf16Index = 0
  var remaining = offset
  while (utf16Index < length && remaining > 0) {
    utf16Index += if (isHighSurrogateAt(utf16Index)) 2 else 1
    remaining--
  }
  return utf16Index.coerceAtMost(length)
}

private fun String.isHighSurrogateAt(index: Int): Boolean =
  this[index] in '\uD800'..'\uDBFF' && index + 1 < length && this[index + 1] in '\uDC00'..'\uDFFF'
