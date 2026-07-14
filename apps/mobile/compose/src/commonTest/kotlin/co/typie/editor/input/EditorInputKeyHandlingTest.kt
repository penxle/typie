package co.typie.editor.input

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.key.Key
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorInputKeyHandlingTest {
  @Test
  fun `Android navigation keys commit composition before running their binding`() {
    val navigationKeys =
      listOf(
        Key.DirectionLeft,
        Key.DirectionRight,
        Key.DirectionUp,
        Key.DirectionDown,
        Key.MoveHome,
        Key.MoveEnd,
        Key.PageUp,
        Key.PageDown,
      )

    for (key in navigationKeys) {
      assertTrue(commitsCompositionBeforeKeyBinding(platform = Platform.Android, key = key))
    }
  }

  @Test
  fun `Android editing keys stay blocked during composition`() {
    val blockedKeys = listOf(Key.Backspace, Key.Delete, Key.Enter, Key.Tab, Key.Escape, Key.A)

    for (key in blockedKeys) {
      assertFalse(commitsCompositionBeforeKeyBinding(platform = Platform.Android, key = key))
    }
  }

  @Test
  fun `non-Android platforms keep navigation keys blocked during composition`() {
    assertFalse(
      commitsCompositionBeforeKeyBinding(platform = Platform.Desktop, key = Key.DirectionLeft)
    )
    assertFalse(
      commitsCompositionBeforeKeyBinding(platform = Platform.iOS, key = Key.DirectionLeft)
    )
  }

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
  fun `fixed local caret text field rect keeps caret origin and expands to clipping edge`() {
    assertEquals(
      Rect(left = 100f, top = 200f, right = 360f, bottom = 218f),
      fixedLocalCaretTextFieldRectInRoot(
        focusedRectInRoot = Rect(left = 100f, top = 200f, right = 101f, bottom = 218f),
        textClippingRectInRoot = Rect(left = 40f, top = 120f, right = 360f, bottom = 700f),
        fallbackRectInRoot = Rect(left = 20f, top = 80f, right = 380f, bottom = 720f),
      ),
    )
  }

  @Test
  fun `fixed local caret text field rect falls back when cursor is unknown`() {
    val fallback = Rect(left = 20f, top = 80f, right = 380f, bottom = 720f)

    assertEquals(
      fallback,
      fixedLocalCaretTextFieldRectInRoot(
        focusedRectInRoot = null,
        textClippingRectInRoot = Rect(left = 40f, top = 120f, right = 360f, bottom = 700f),
        fallbackRectInRoot = fallback,
      ),
    )
  }

  @Test
  fun `input session restarts when suppression changes on platforms that require restart`() {
    assertTrue(
      shouldRestartEditorInputSession(
        previousEnabled = true,
        enabled = true,
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
        previousEnabled = true,
        enabled = true,
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
        previousEnabled = false,
        enabled = true,
        previousSuppressSoftwareKeyboard = true,
        suppressSoftwareKeyboard = true,
        restartOnSoftwareKeyboardSuppressionChange = false,
      )
    )
  }
}
