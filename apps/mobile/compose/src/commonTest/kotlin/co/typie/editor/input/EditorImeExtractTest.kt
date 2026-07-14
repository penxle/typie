package co.typie.editor.input

import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeExtractTest {
  @Test
  fun `extract reports window text with window-relative utf16 selection`() {
    val ime = Ime(text = "가나다라", windowStart = 0, selection = ImeRange(2, 3), composing = null)

    val extract = ime.extract()

    assertEquals("가나다라", extract.text)
    assertEquals(0, extract.startOffset)
    assertEquals(2, extract.selectionStart)
    assertEquals(3, extract.selectionEnd)
  }

  @Test
  fun `extract keeps flat window start for windowed documents`() {
    val ime = Ime(text = "다라마", windowStart = 100, selection = ImeRange(101, 101), composing = null)

    val extract = ime.extract()

    assertEquals(100, extract.startOffset)
    assertEquals(1, extract.selectionStart)
    assertEquals(1, extract.selectionEnd)
  }

  @Test
  fun `extract counts surrogate pairs in utf16 selection offsets`() {
    // flat: a=100, 😀=101, 가=102, 나=103
    val ime =
      Ime(text = "a😀가나", windowStart = 100, selection = ImeRange(103, 104), composing = null)

    val extract = ime.extract()

    assertEquals(4, extract.selectionStart) // a(1) + 😀(2) + 가(1)
    assertEquals(5, extract.selectionEnd)
  }

  @Test
  fun `extract absolute offsets roundtrip through the incoming projection`() {
    val ime =
      Ime(text = "a😀가나", windowStart = 100, selection = ImeRange(103, 104), composing = null)

    val extract = ime.extract()

    assertEquals(
      ime.selection.start,
      ime.projectAbsoluteUtf16Offset(extract.startOffset + extract.selectionStart),
    )
    assertEquals(
      ime.selection.end,
      ime.projectAbsoluteUtf16Offset(extract.startOffset + extract.selectionEnd),
    )
  }

  @Test
  fun `absolute utf16 offset is the inverse of the incoming projection`() {
    val ime =
      Ime(text = "a😀가나", windowStart = 100, selection = ImeRange(100, 104), composing = null)

    for (flat in 100..104) {
      assertEquals(flat, ime.projectAbsoluteUtf16Offset(ime.absoluteUtf16Offset(flat)))
    }
  }
}
