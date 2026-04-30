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
}
