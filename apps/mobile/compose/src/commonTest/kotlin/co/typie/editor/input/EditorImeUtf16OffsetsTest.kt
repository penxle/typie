package co.typie.editor.input

import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeUtf16OffsetsTest {
  @Test
  fun `window utf16 offset without surrogates equals the window-relative flat offset`() {
    val ime = Ime(text = "abc", windowStart = 100, selection = ImeRange(103, 103), composing = null)

    assertEquals(2, ime.windowUtf16Offset(102))
  }

  @Test
  fun `window utf16 offset counts surrogate pairs`() {
    // flat: a=10, 😀=11, b=12
    val ime = Ime(text = "a😀b", windowStart = 10, selection = ImeRange(13, 13), composing = null)

    assertEquals(1, ime.windowUtf16Offset(11))
    assertEquals(3, ime.windowUtf16Offset(12))
    assertEquals(4, ime.windowUtf16Offset(13))
  }

  @Test
  fun `window utf16 offset clamps outside the window`() {
    val ime = Ime(text = "abc", windowStart = 100, selection = ImeRange(103, 103), composing = null)

    assertEquals(0, ime.windowUtf16Offset(50))
    assertEquals(3, ime.windowUtf16Offset(200))
  }

  @Test
  fun `window utf16 offset is the inverse of the incoming projection`() {
    val ime =
      Ime(text = "a😀가나", windowStart = 100, selection = ImeRange(100, 104), composing = null)

    for (flat in 100..104) {
      assertEquals(flat, ime.projectWindowUtf16Index(ime.windowUtf16Offset(flat)))
    }
  }
}
