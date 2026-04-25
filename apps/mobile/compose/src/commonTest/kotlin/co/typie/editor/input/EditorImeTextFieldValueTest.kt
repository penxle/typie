package co.typie.editor.input

import androidx.compose.ui.text.TextRange
import co.typie.editor.ffi.Ime
import co.typie.editor.ffi.ImeRange
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorImeTextFieldValueTest {
  @Test
  fun ime_context_selection_is_clamped_to_text_length() {
    val value =
      Ime(text = "abc", windowStart = 0, selection = ImeRange(start = 4, end = 4), composing = null)
        .toTextFieldValue()

    assertEquals(TextRange(3), value.selection)
  }

  @Test
  fun ime_context_flat_offsets_are_mapped_to_utf16_indices() {
    val value =
      Ime(
          text = "a\uD83D\uDE00b",
          windowStart = 0,
          selection = ImeRange(start = 2, end = 3),
          composing = ImeRange(start = 1, end = 3),
        )
        .toTextFieldValue()

    assertEquals(TextRange(start = 3, end = 4), value.selection)
    assertEquals(TextRange(start = 1, end = 4), value.composition)
  }
}
