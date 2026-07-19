package co.typie.ext

// cspell:ignore heyo

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class TextInputStateTest {

  @Test
  fun resolve_text_input_change_ignores_selection_only_updates() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(5))
    val nextValue = TextFieldValue(text = "hello", selection = TextRange(2, 4))

    val result = resolveTextInputChange(currentValue = currentValue, nextValue = nextValue)

    assertFalse(result.textChanged)
    assertEquals(nextValue, result.nextValue)
  }

  @Test
  fun resolve_text_input_change_ignores_composition_only_updates() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(5))
    val nextValue =
      TextFieldValue(text = "hello", selection = TextRange(5), composition = TextRange(0, 2))

    val result = resolveTextInputChange(currentValue = currentValue, nextValue = nextValue)

    assertFalse(result.textChanged)
    assertEquals(nextValue, result.nextValue)
  }

  @Test
  fun resolve_text_input_change_reports_text_updates() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(5))
    val nextValue = TextFieldValue(text = "hello!", selection = TextRange(6))

    val result = resolveTextInputChange(currentValue = currentValue, nextValue = nextValue)

    assertTrue(result.textChanged)
    assertEquals(nextValue, result.nextValue)
  }

  @Test
  fun sync_text_input_value_clamps_selection_to_new_text_length() {
    val currentValue =
      TextFieldValue(
        text = "hello world",
        selection = TextRange(6, 11),
        composition = TextRange(6, 11),
      )

    val result = syncTextInputValue(currentValue = currentValue, text = "hello")

    assertEquals(
      TextFieldValue(text = "hello", selection = TextRange(5), composition = null),
      result,
    )
  }

  @Test
  fun sync_text_input_value_keeps_cursor_at_text_end_when_external_text_grows() {
    val currentValue = TextFieldValue(text = "", selection = TextRange.Zero)

    val result = syncTextInputValue(currentValue = currentValue, text = "recent search")

    assertEquals(
      TextFieldValue(text = "recent search", selection = TextRange("recent search".length)),
      result,
    )
  }

  @Test
  fun sync_text_input_value_keeps_middle_selection_when_external_text_grows() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(1, 4))

    val result = syncTextInputValue(currentValue = currentValue, text = "hello world")

    assertEquals(TextFieldValue(text = "hello world", selection = TextRange(1, 4)), result)
  }

  @Test
  fun sync_text_input_value_keeps_state_when_text_matches() {
    val currentValue =
      TextFieldValue(text = "hello", selection = TextRange(1, 4), composition = TextRange(1, 4))

    val result = syncTextInputValue(currentValue = currentValue, text = "hello")

    assertEquals(currentValue, result)
  }

  @Test
  fun insert_text_input_value_replaces_selection() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(1, 4))

    val result = insertTextInputValue(currentValue = currentValue, text = "i")

    assertEquals(TextFieldValue(text = "hio", selection = TextRange(2)), result)
  }

  @Test
  fun set_composing_text_input_value_replaces_active_range_and_marks_composition() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(1, 4))

    val result = setComposingTextInputValue(currentValue = currentValue, text = "ey")

    assertEquals(
      TextFieldValue(text = "heyo", selection = TextRange(3), composition = TextRange(1, 3)),
      result,
    )
  }

  @Test
  fun commit_text_input_value_replaces_existing_composition() {
    val currentValue =
      TextFieldValue(text = "heyo", selection = TextRange(3), composition = TextRange(1, 3))

    val result = commitTextInputValue(currentValue = currentValue, text = "ell")

    assertEquals(TextFieldValue(text = "hello", selection = TextRange(4)), result)
  }

  @Test
  fun finish_text_input_composition_only_clears_composition() {
    val currentValue =
      TextFieldValue(text = "hello", selection = TextRange(5), composition = TextRange(1, 4))

    val result = finishTextInputComposition(currentValue = currentValue)

    assertEquals(TextFieldValue(text = "hello", selection = TextRange(5)), result)
  }

  @Test
  fun delete_backward_text_input_value_removes_active_composition() {
    val currentValue =
      TextFieldValue(text = "hello", selection = TextRange(4), composition = TextRange(1, 4))

    val result = deleteBackwardTextInputValue(currentValue = currentValue)

    assertEquals(TextFieldValue(text = "ho", selection = TextRange(1)), result)
  }

  @Test
  fun delete_backward_text_input_value_removes_previous_character_when_selection_is_collapsed() {
    val currentValue = TextFieldValue(text = "hello", selection = TextRange(5))

    val result = deleteBackwardTextInputValue(currentValue = currentValue)

    assertEquals(TextFieldValue(text = "hell", selection = TextRange(4)), result)
  }
}
