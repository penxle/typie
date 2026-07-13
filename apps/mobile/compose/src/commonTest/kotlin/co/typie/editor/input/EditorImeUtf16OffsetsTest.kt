package co.typie.editor.input

import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeUtf16OffsetsTest {
  @Test
  fun `absolute utf16 offset without surrogates is unchanged`() {
    val ime = Ime(text = "abc", windowStart = 100, selection = ImeRange(103, 103), composing = null)

    assertEquals(102, ime.projectAbsoluteUtf16Offset(102))
  }

  @Test
  fun `absolute utf16 offset within window converts utf16 units to code points`() {
    val ime = Ime(text = "a😀b", windowStart = 10, selection = ImeRange(13, 13), composing = null)

    assertEquals(11, ime.projectAbsoluteUtf16Offset(11))
    assertEquals(12, ime.projectAbsoluteUtf16Offset(13))
    assertEquals(13, ime.projectAbsoluteUtf16Offset(14))
  }

  @Test
  fun `absolute utf16 offset outside window passes through`() {
    val ime = Ime(text = "abc", windowStart = 100, selection = ImeRange(103, 103), composing = null)

    assertEquals(50, ime.projectAbsoluteUtf16Offset(50))
    assertEquals(104, ime.projectAbsoluteUtf16Offset(104))
  }
}
