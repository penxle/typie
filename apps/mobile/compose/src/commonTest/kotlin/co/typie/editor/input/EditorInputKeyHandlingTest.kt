package co.typie.editor.input

import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.input.key.Key
import co.typie.editor.KeyModifier
import co.typie.editor.createBindings
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorInputKeyHandlingTest {
  @Test
  fun `toolbar insert text commits composition before inserting while composing`() {
    assertEquals(
      listOf(
        Message.TextInput(listOf(FlatImeOp.CommitAsIs)),
        Message.Insertion(InsertionOp.Text("가")),
      ),
      toolbarInsertTextMessages(text = "가", composing = true),
    )
  }

  @Test
  fun `toolbar insert text inserts directly without composition`() {
    assertEquals(
      listOf(Message.Insertion(InsertionOp.Text("가"))),
      toolbarInsertTextMessages(text = "가", composing = false),
    )
  }

  @Test
  fun `navigation bindings commit composition on every platform`() {
    val navigationBindings =
      listOf(
        Key.DirectionLeft to emptySet(),
        Key.DirectionLeft to setOf(KeyModifier.Shift),
        Key.DirectionLeft to setOf(KeyModifier.Alt),
        Key.DirectionLeft to setOf(KeyModifier.Mod),
        Key.DirectionDown to setOf(KeyModifier.Mod, KeyModifier.Shift),
        Key.MoveHome to emptySet(),
        Key.MoveEnd to setOf(KeyModifier.Ctrl),
        Key.PageUp to setOf(KeyModifier.Shift),
        Key.PageDown to emptySet(),
      )

    for (platform in Platform.entries) {
      for ((key, modifiers) in navigationBindings) {
        assertTrue(
          binding(platform, key, modifiers).commitCompositionBeforeDispatch,
          "$platform $key $modifiers",
        )
      }
    }
  }

  @Test
  fun `editor shortcuts commit composition on every platform`() {
    val shortcutBindings =
      listOf(
        Key.Enter to setOf(KeyModifier.Mod),
        Key.A to setOf(KeyModifier.Mod),
        Key.B to setOf(KeyModifier.Mod),
        Key.Z to setOf(KeyModifier.Mod),
        Key.C to setOf(KeyModifier.Mod),
        Key.V to setOf(KeyModifier.Mod),
      )

    for (platform in Platform.entries) {
      for ((key, modifiers) in shortcutBindings) {
        assertTrue(
          binding(platform, key, modifiers).commitCompositionBeforeDispatch,
          "$platform $key $modifiers",
        )
      }
    }
  }

  @Test
  fun `native text input bindings stay blocked during composition`() {
    val nativeBindings =
      listOf(
        Key.Enter to emptySet(),
        Key.Enter to setOf(KeyModifier.Shift),
        Key.Backspace to emptySet(),
        Key.Backspace to setOf(KeyModifier.Alt),
        Key.Backspace to setOf(KeyModifier.Ctrl),
        Key.Backspace to setOf(KeyModifier.Mod),
        Key.Delete to emptySet(),
        Key.Delete to setOf(KeyModifier.Alt),
        Key.Delete to setOf(KeyModifier.Ctrl),
        Key.Tab to emptySet(),
        Key.Tab to setOf(KeyModifier.Shift),
        Key.Escape to emptySet(),
      )

    for (platform in Platform.entries) {
      for ((key, modifiers) in nativeBindings) {
        assertFalse(
          binding(platform, key, modifiers).commitCompositionBeforeDispatch,
          "$platform $key $modifiers",
        )
      }
    }
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

  private fun binding(platform: Platform, key: Key, modifiers: Set<KeyModifier>) =
    createBindings(platform).first { it.key == key && it.modifiers == modifiers }
}
