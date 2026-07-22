package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorKeyboardTypeDesktopTest {
  @Test
  fun desktop_software_keyboard_uses_current_ime_bottom() {
    val state =
      resolveDesktopEditorKeyboardState(hardwareKeyboardConnected = false, imeBottom = 320.dp)

    assertEquals(EditorKeyboardType.Software, state.type)
    assertEquals(true, state.imeFrameVisible)
    assertEquals(EditorKeyboardPresentation.Shown(settledImeBottom = 320.dp), state.presentation)
    assertEquals(320.dp, state.settledImeBottom)
  }

  @Test
  fun desktop_hardware_keyboard_ignores_debug_ime_bottom() {
    val state =
      resolveDesktopEditorKeyboardState(hardwareKeyboardConnected = true, imeBottom = 320.dp)

    assertEquals(EditorKeyboardType.Hardware, state.type)
    assertEquals(false, state.imeFrameVisible)
    assertEquals(EditorKeyboardPresentation.Hidden, state.presentation)
  }

  @Test
  fun desktop_feed_reuses_visible_auxiliary_owner_when_editor_refocuses_at_hide() {
    val tracker = EditorImeHideOwnershipTracker()
    val visible =
      resolveDesktopEditorKeyboardState(hardwareKeyboardConnected = false, imeBottom = 320.dp)
    val hidden =
      resolveDesktopEditorKeyboardState(hardwareKeyboardConnected = false, imeBottom = 0.dp)

    assertEquals(
      null,
      tracker.observe(presentation = visible.presentation, editorInputSessionActive = false),
    )
    assertEquals(
      EditorImeInputOwner.Other,
      tracker.observe(presentation = hidden.presentation, editorInputSessionActive = true),
    )
  }
}
