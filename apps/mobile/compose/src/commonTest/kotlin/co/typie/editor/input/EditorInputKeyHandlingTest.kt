package co.typie.editor.input

import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorInputKeyHandlingTest {
  @Test
  fun `iOS printable text is always owned by platform text input`() {
    assertFalse(requiresRawKeyTextFallback(platform = Platform.iOS))
  }

  @Test
  fun `non iOS platforms keep raw key text fallback`() {
    assertTrue(requiresRawKeyTextFallback(platform = Platform.Android))
    assertTrue(requiresRawKeyTextFallback(platform = Platform.Desktop))
  }

  @Test
  fun `input session restarts when suppression changes on platforms that require restart`() {
    assertTrue(
      shouldRestartEditorInputSession(
        previousTextInputSessionEnabled = true,
        textInputSessionEnabled = true,
        previousSuppressSoftwareKeyboard = false,
        suppressSoftwareKeyboard = true,
        restartOnSoftwareKeyboardSuppressionChange = true,
      )
    )
  }

  @Test
  fun `input session does not restart for suppression-only change when platform can hide keyboard surface`() {
    assertFalse(
      shouldRestartEditorInputSession(
        previousTextInputSessionEnabled = true,
        textInputSessionEnabled = true,
        previousSuppressSoftwareKeyboard = false,
        suppressSoftwareKeyboard = true,
        restartOnSoftwareKeyboardSuppressionChange = false,
      )
    )
  }

  @Test
  fun `input session restarts when enabled state changes`() {
    assertTrue(
      shouldRestartEditorInputSession(
        previousTextInputSessionEnabled = false,
        textInputSessionEnabled = true,
        previousSuppressSoftwareKeyboard = true,
        suppressSoftwareKeyboard = true,
        restartOnSoftwareKeyboardSuppressionChange = false,
      )
    )
  }
}
