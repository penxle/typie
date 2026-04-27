package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarBottomStateTest {
  @Test
  fun hardware_keyboard_without_software_keyboard_uses_hide_toolbar_action() {
    val action =
      resolveEditorToolbarFixedAction(
        activeBottomPanel = null,
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      )

    assertEquals(EditorToolbarFixedAction.HideToolbar, action)
  }

  @Test
  fun hardware_keyboard_without_software_keyboard_keeps_hide_toolbar_action_after_focus_clears() {
    val action =
      resolveEditorToolbarFixedAction(
        activeBottomPanel = null,
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      )

    assertEquals(EditorToolbarFixedAction.HideToolbar, action)
  }

  @Test
  fun software_keyboard_uses_dismiss_input_action() {
    val action =
      resolveEditorToolbarFixedAction(
        activeBottomPanel = null,
        keyboardType = EditorKeyboardType.Software,
        softwareKeyboardVisible = true,
      )

    assertEquals(EditorToolbarFixedAction.DismissInput, action)
  }

  @Test
  fun open_bottom_panel_uses_close_panel_action() {
    val action =
      resolveEditorToolbarFixedAction(
        activeBottomPanel = EditorToolbarBottomPanelKey.Insert,
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      )

    assertEquals(EditorToolbarFixedAction.ClosePanel, action)
  }

  @Test
  fun hardware_keyboard_bottom_panel_uses_fallback_reserved_inset() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(252.dp, state.visibleImeInset(imeBottom = 0.dp, safeBottomInset = 24.dp))
    assertEquals(220.dp, state.bottomPanelHeight(safeBottomInset = 24.dp))
  }

  @Test
  fun open_bottom_panel_keeps_text_input_session_enabled_for_hardware_keyboard() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(true, state.textInputSessionEnabled(EditorKeyboardType.Hardware))
  }

  @Test
  fun open_bottom_panel_disables_text_input_session_for_software_keyboard() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(false, state.textInputSessionEnabled(EditorKeyboardType.Software))
  }

  @Test
  fun visible_bottom_panel_layout_height_includes_gap_and_panel() {
    assertEquals(
      228.dp,
      resolveEditorToolbarBottomPanelLayoutHeight(
        bottomPanelVisible = true,
        bottomPanelHeight = 220.dp,
      ),
    )
  }

  @Test
  fun hidden_bottom_panel_layout_height_collapses_to_zero() {
    assertEquals(
      0.dp,
      resolveEditorToolbarBottomPanelLayoutHeight(
        bottomPanelVisible = false,
        bottomPanelHeight = 220.dp,
      ),
    )
  }
}
