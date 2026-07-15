package co.typie.editor.input

import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeComposingRegionTest {
  private val window = "가".repeat(8192)

  @Test
  fun `zero-length region clears composition`() {
    val ime = Ime(text = "가나다", windowStart = 0, selection = ImeRange(3, 3), composing = null)

    assertEquals(ComposingRegionDecision.Clear, resolveComposingRegion(ime, 2, 2))
  }

  @Test
  fun `missing ime context clears composition`() {
    assertEquals(ComposingRegionDecision.Clear, resolveComposingRegion(null, 1, 3))
  }

  @Test
  fun `reversed region swaps to ordered range`() {
    val ime = Ime(text = "가나다라마", windowStart = 0, selection = ImeRange(5, 5), composing = null)

    assertEquals(ComposingRegionDecision.Set(3, 5), resolveComposingRegion(ime, 5, 3))
  }

  @Test
  fun `window-relative region from a windowed document projects to the composed text`() {
    // log5: cursor at flat 8513 in a large document (window base 4417), active
    // composition on the just-typed syllable at flat 8512..8513. The keyboard
    // derives its offsets from the window-relative world (4095..4096).
    val ime =
      Ime(
        text = window,
        windowStart = 4417,
        selection = ImeRange(8513, 8513),
        composing = ImeRange(8512, 8513),
      )

    assertEquals(ComposingRegionDecision.Set(8512, 8513), resolveComposingRegion(ime, 4095, 4096))
  }

  @Test
  fun `window-relative region resolves even where an absolute reading would also fit`() {
    // Cursor at flat 4500 (window base 404): the same keyboard math produces
    // 4095..4096, which under an absolute reading would land 404 characters
    // early; the window-relative projection recovers the true target.
    val ime =
      Ime(
        text = window,
        windowStart = 404,
        selection = ImeRange(4500, 4500),
        composing = ImeRange(4499, 4500),
      )

    assertEquals(ComposingRegionDecision.Set(4499, 4500), resolveComposingRegion(ime, 4095, 4096))
  }

  @Test
  fun `region far from selection and composition clears composition`() {
    val ime =
      Ime(text = window, windowStart = 0, selection = ImeRange(8000, 8000), composing = null)

    assertEquals(ComposingRegionDecision.Clear, resolveComposingRegion(ime, 10, 12))
  }

  @Test
  fun `region within slack of the selection is honored`() {
    val ime = Ime(text = window, windowStart = 0, selection = ImeRange(100, 100), composing = null)

    assertEquals(ComposingRegionDecision.Set(40, 60), resolveComposingRegion(ime, 40, 60))
  }

  @Test
  fun `region near the active composition is honored even far from the selection`() {
    val ime =
      Ime(
        text = window,
        windowStart = 0,
        selection = ImeRange(500, 500),
        composing = ImeRange(100, 103),
      )

    assertEquals(ComposingRegionDecision.Set(98, 103), resolveComposingRegion(ime, 98, 103))
  }

  @Test
  fun `region offsets count surrogate pairs`() {
    // flat: a=10, 😀=11, 가=12, 나=13
    val ime = Ime(text = "a😀가나", windowStart = 10, selection = ImeRange(13, 13), composing = null)

    assertEquals(ComposingRegionDecision.Set(11, 12), resolveComposingRegion(ime, 1, 3))
  }
}
