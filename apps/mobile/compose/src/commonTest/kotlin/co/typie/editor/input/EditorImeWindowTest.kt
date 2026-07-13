package co.typie.editor.input

import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertSame

class EditorImeWindowTest {
  @Test
  fun trimsBothSidesAroundSelection() {
    val ime =
      Ime(text = "abcdefg", windowStart = 10, selection = ImeRange(13, 14), composing = null)

    val trimmed = ime.trimmedTo(beforeLimit = 1, afterLimit = 1)

    assertEquals("cde", trimmed.text)
    assertEquals(12, trimmed.windowStart)
    assertEquals(ImeRange(13, 14), trimmed.selection)
  }

  @Test
  fun zeroLimitsKeepOnlySelectedText() {
    val ime =
      Ime(text = "abcdefg", windowStart = 10, selection = ImeRange(13, 15), composing = null)

    val trimmed = ime.trimmedTo(beforeLimit = 0, afterLimit = 0)

    assertEquals("de", trimmed.text)
    assertEquals(13, trimmed.windowStart)
  }

  @Test
  fun limitsBeyondWindowReturnSameInstance() {
    val ime =
      Ime(
        text = "abcdefg",
        windowStart = 10,
        selection = ImeRange(13, 14),
        composing = ImeRange(12, 14),
      )

    assertSame(ime, ime.trimmedTo(beforeLimit = 4096, afterLimit = 4096))
  }

  @Test
  fun trimsAtSurrogatePairBoundaries() {
    val text = "😀😀a😀"
    val ime = Ime(text = text, windowStart = 0, selection = ImeRange(2, 3), composing = null)

    val trimmed = ime.trimmedTo(beforeLimit = 1, afterLimit = 1)

    assertEquals("😀a😀", trimmed.text)
    assertEquals(1, trimmed.windowStart)
  }

  @Test
  fun clampsWhenSelectionSpansWholeWindow() {
    val ime = Ime(text = "abc", windowStart = 0, selection = ImeRange(0, 3), composing = null)

    val trimmed = ime.trimmedTo(beforeLimit = 5, afterLimit = 5)

    assertEquals("abc", trimmed.text)
    assertEquals(0, trimmed.windowStart)
  }
}
