package co.typie.ui.component

import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class TextFieldKeyHandlingTest {

  @Test
  fun tab_preview_key_down_is_consumed_and_suppresses_tab_value_change() {
    assertEquals(
      TextFieldPreviewKeyResult(
        tabAction = TextFieldTabAction.Next,
        triggerImeAction = false,
        consumeEvent = true,
        suppressTabValueChange = true,
      ),
      resolveTextFieldPreviewKeyResult(
        key = Key.Tab,
        type = KeyEventType.KeyDown,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun shift_tab_preview_key_down_is_consumed_and_suppresses_tab_value_change() {
    assertEquals(
      TextFieldPreviewKeyResult(
        tabAction = TextFieldTabAction.Previous,
        triggerImeAction = false,
        consumeEvent = true,
        suppressTabValueChange = true,
      ),
      resolveTextFieldPreviewKeyResult(
        key = Key.Tab,
        type = KeyEventType.KeyDown,
        isShiftPressed = true,
      )
    )
  }

  @Test
  fun enter_preview_key_down_triggers_ime_without_tab_suppression() {
    assertEquals(
      TextFieldPreviewKeyResult(
        tabAction = null,
        triggerImeAction = true,
        consumeEvent = false,
        suppressTabValueChange = false,
      ),
      resolveTextFieldPreviewKeyResult(
        key = Key.Enter,
        type = KeyEventType.KeyDown,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun shift_tab_key_down_triggers_previous_tab_action() {
    assertEquals(
      TextFieldTabAction.Previous,
      resolveTextFieldTabAction(
        key = Key.Tab,
        type = KeyEventType.KeyDown,
        isShiftPressed = true,
      )
    )
  }

  @Test
  fun tab_key_down_triggers_next_tab_action() {
    assertEquals(
      TextFieldTabAction.Next,
      resolveTextFieldTabAction(
        key = Key.Tab,
        type = KeyEventType.KeyDown,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun enter_key_down_does_not_trigger_tab_action() {
    assertEquals<TextFieldTabAction?>(
      null,
      resolveTextFieldTabAction(
        key = Key.Enter,
        type = KeyEventType.KeyDown,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun enter_key_down_triggers_ime_action() {
    assertTrue(
      shouldHandleTextFieldImeAction(
        key = Key.Enter,
        type = KeyEventType.KeyDown,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun tab_key_down_does_not_trigger_ime_action() {
    assertFalse(
      shouldHandleTextFieldImeAction(
        key = Key.Tab,
        type = KeyEventType.KeyDown,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun shift_tab_key_down_does_not_trigger_ime_action() {
    assertFalse(
      shouldHandleTextFieldImeAction(
        key = Key.Tab,
        type = KeyEventType.KeyDown,
        isShiftPressed = true,
      )
    )
  }

  @Test
  fun enter_key_up_does_not_trigger_ime_action() {
    assertFalse(
      shouldHandleTextFieldImeAction(
        key = Key.Enter,
        type = KeyEventType.KeyUp,
        isShiftPressed = false,
      )
    )
  }

  @Test
  fun tab_character_value_change_triggers_tab_action_when_available() {
    val currentValue = TextFieldValue(
      text = "user@example.com",
      selection = TextRange("user@example.com".length),
    )
    val newValue = TextFieldValue(
      text = "user@example.com\t",
      selection = TextRange("user@example.com\t".length),
    )

    val result = resolveTextFieldValueChange(
      currentValue = currentValue,
      newValue = newValue,
      tabNavigationEnabled = true,
      hasTabAction = true,
      suppressTabValueChange = false,
    )

    assertTrue(result.triggerTabAction)
    assertTrue(result.consumeValueChange)
    assertFalse(result.suppressTabValueChange)
    assertEquals(currentValue, result.nextValue)
  }

  @Test
  fun tab_character_value_change_is_consumed_without_next_action_when_navigation_is_enabled() {
    val currentValue = TextFieldValue(
      text = "user@example.com",
      selection = TextRange("user@example.com".length),
    )
    val newValue = TextFieldValue(
      text = "user@example.com\t",
      selection = TextRange("user@example.com\t".length),
    )

    val result = resolveTextFieldValueChange(
      currentValue = currentValue,
      newValue = newValue,
      tabNavigationEnabled = true,
      hasTabAction = false,
      suppressTabValueChange = false,
    )

    assertFalse(result.triggerTabAction)
    assertTrue(result.consumeValueChange)
    assertFalse(result.suppressTabValueChange)
    assertEquals(currentValue, result.nextValue)
  }

  @Test
  fun suppressed_tab_value_change_is_ignored_without_triggering_navigation() {
    val currentValue = TextFieldValue(
      text = "user@example.com",
      selection = TextRange("user@example.com".length),
    )
    val newValue = TextFieldValue(
      text = "user@example.com\t",
      selection = TextRange("user@example.com\t".length),
    )

    val result = resolveTextFieldValueChange(
      currentValue = currentValue,
      newValue = newValue,
      tabNavigationEnabled = true,
      hasTabAction = true,
      suppressTabValueChange = true,
    )

    assertFalse(result.triggerTabAction)
    assertTrue(result.consumeValueChange)
    assertTrue(result.suppressTabValueChange)
    assertEquals(currentValue, result.nextValue)
  }

  @Test
  fun regular_value_change_clears_tab_suppression() {
    val currentValue = TextFieldValue(
      text = "user",
      selection = TextRange("user".length),
    )
    val newValue = TextFieldValue(
      text = "users",
      selection = TextRange("users".length),
    )

    val result = resolveTextFieldValueChange(
      currentValue = currentValue,
      newValue = newValue,
      tabNavigationEnabled = true,
      hasTabAction = true,
      suppressTabValueChange = true,
    )

    assertFalse(result.triggerTabAction)
    assertFalse(result.consumeValueChange)
    assertFalse(result.suppressTabValueChange)
    assertEquals(newValue, result.nextValue)
  }
}
