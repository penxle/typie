package co.typie.editor.input

import androidx.compose.ui.geometry.Rect
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionOp
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertEquals
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

  @Test
  fun `selection echo tracker consumes only matching horizontal echo`() {
    val tracker = EditorInputSelectionEchoTracker()

    assertFalse(tracker.consumeIfEcho(selectionDelta(delta = 1)))

    tracker.expect(
      direction = EditorInputSelectionEchoDirection.Forward,
      selection = ImeRange(1, 1),
      extend = false,
    )
    assertFalse(tracker.consumeIfEcho(selectionDelta(delta = -1)))
    assertTrue(tracker.consumeIfEcho(selectionDelta(delta = 1)))
    assertFalse(tracker.consumeIfEcho(selectionDelta(delta = 1)))
  }

  @Test
  fun `selection echo tracker can expire pending echoes`() {
    val tracker = EditorInputSelectionEchoTracker()

    val echo =
      tracker.expect(
        direction = EditorInputSelectionEchoDirection.Forward,
        selection = ImeRange(1, 1),
        extend = false,
      )
    tracker.expire(echo)

    assertFalse(tracker.consumeIfEcho(selectionDelta(delta = 1)))
  }

  @Test
  fun `expired selection echo does not consume a newer echo`() {
    val tracker = EditorInputSelectionEchoTracker()

    val stale =
      tracker.expect(
        direction = EditorInputSelectionEchoDirection.Forward,
        selection = ImeRange(1, 1),
        extend = false,
      )
    assertTrue(tracker.consumeIfEcho(selectionDelta(delta = 1)))
    tracker.expect(
      direction = EditorInputSelectionEchoDirection.Forward,
      selection = ImeRange(2, 2),
      extend = false,
    )
    tracker.expire(stale)

    assertTrue(tracker.consumeIfEcho(selectionDelta(delta = 1)))
  }

  @Test
  fun `plain selection echo does not consume absolute IME selection`() {
    val tracker = EditorInputSelectionEchoTracker()

    tracker.expect(
      direction = EditorInputSelectionEchoDirection.Forward,
      selection = ImeRange(1, 1),
      extend = false,
    )

    assertFalse(tracker.consumeIfEcho(selectionSet(start = 1, end = 4)))
  }

  @Test
  fun `extended selection echo consumes directional range selection`() {
    val tracker = EditorInputSelectionEchoTracker()

    tracker.expect(
      direction = EditorInputSelectionEchoDirection.Forward,
      selection = ImeRange(1, 1),
      extend = true,
    )

    assertTrue(tracker.consumeIfEcho(selectionSet(start = 1, end = 2)))
  }

  private fun selectionDelta(delta: Int): List<Message> {
    val direction = if (delta > 0) Direction.Forward else Direction.Backward
    return List(kotlin.math.abs(delta)) {
      Message.Navigation(NavigationOp.Move(Movement.Grapheme(direction), false))
    }
  }

  private fun selectionSet(start: Int, end: Int): List<Message> =
    listOf(Message.Selection(SelectionOp.SetFlat(start = start, end = end)))
}
