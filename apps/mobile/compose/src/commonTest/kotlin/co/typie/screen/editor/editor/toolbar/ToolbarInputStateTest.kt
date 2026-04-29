package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarInputStateTest {
  @Test
  fun software_keyboard_panel_open_close_restores_keyboard_without_height_collapse() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)

    assertEquals(listOf(ToolbarEffect.HideKeyboard), state.takeEffects())
    assertEquals(
      PanelSession(
        key = EditorToolbarBottomPanelKey.Insert,
        height = 288.dp,
        keyboardSpace = KeyboardSpaceAnchor(inset = 320.dp),
      ),
      state.panel,
    )
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

    assertEquals(
      listOf(ToolbarEffect.RequestFocus, ToolbarEffect.ShowKeyboard),
      state.takeEffects(),
    )
    assertEquals(320.dp, state.keyboardRestore?.anchor?.inset)
    assertEquals(null, state.panel)
    assertEquals(EditorToolbarBottomPanelKey.Insert, state.lastBottomPanel)

    val restoring = toolbarInputEnvironment(imeBottom = 95.dp, panelTransitionIdle = false)
    state.onEnvironmentChanged(restoring)

    assertEquals(320.dp, state.keyboardRestore?.anchor?.inset)
    assertEquals(288.dp, state.lastBottomPanelHeight)

    val restored = toolbarInputEnvironment(imeBottom = 320.dp, panelTransitionIdle = true)
    state.onEnvironmentChanged(restored)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestore)
    assertEquals(320.dp, effectiveImeInset(restored))
  }

  @Test
  fun rapid_toggle_switches_panel_while_panel_transition_is_running() {
    val state = EditorToolbarInputState()
    val environment = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(environment)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), environment)
    state.takeEffects()

    val transitioning = environment.copy(panelTransitionIdle = false)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), transitioning)

    assertEquals(EditorToolbarBottomPanelKey.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
    assertEquals(emptyList(), state.takeEffects())
  }

  @Test
  fun opening_panel_while_restoring_keyboard_reuses_restore_inset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.takeEffects()

    val restoring = toolbarInputEnvironment(imeBottom = 120.dp, panelTransitionIdle = false)
    state.onEnvironmentChanged(restoring)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), restoring)

    assertEquals(EditorToolbarBottomPanelKey.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
    assertEquals(listOf(ToolbarEffect.HideKeyboard), state.takeEffects())
  }

  @Test
  fun hardware_keyboard_switch_uses_minimum_panel_height_and_filters_stale_ime() {
    val state = EditorToolbarInputState()
    val software = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(software)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), software)
    state.takeEffects()

    val hardwareHidden =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
      )
    state.onEnvironmentChanged(hardwareHidden)

    assertEquals(180.dp, state.panel?.height)
    assertEquals(null, state.panel?.keyboardSpace)
    assertEquals(false, suppressSoftwareKeyboard(requireNotNull(state.panel)))

    state.dispatch(ToolbarIntent.RestoreEditorInput, hardwareHidden)
    state.takeEffects()

    val staleHardwareIme = hardwareHidden.copy(imeBottom = 320.dp)
    state.onEnvironmentChanged(staleHardwareIme)

    assertEquals(0.dp, effectiveImeInset(staleHardwareIme))

    state.onEnvironmentChanged(hardwareHidden)
    val softwareAgain = toolbarInputEnvironment(imeBottom = 320.dp)
    state.onEnvironmentChanged(softwareAgain)

    assertEquals(320.dp, effectiveImeInset(softwareAgain))
  }

  @Test
  fun hardware_keyboard_switch_with_stale_visible_ime_uses_minimum_panel_height() {
    val state = EditorToolbarInputState()
    val software = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(software)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), software)
    state.takeEffects()

    val staleHardwareIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
      )
    state.onEnvironmentChanged(staleHardwareIme)

    assertEquals(180.dp, state.panel?.height)
    assertEquals(null, state.panel?.keyboardSpace)
    assertEquals(0.dp, effectiveImeInset(staleHardwareIme))

    state.dispatch(ToolbarIntent.RestoreEditorInput, staleHardwareIme)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestore)
    assertEquals(0.dp, state.retainedKeyboardInset())
    assertEquals(listOf(ToolbarEffect.RequestFocus), state.takeEffects())
  }

  @Test
  fun hardware_keyboard_with_visible_ime_uses_minimum_panel_height() {
    val state = EditorToolbarInputState()
    val environment =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
      )

    state.onEnvironmentChanged(environment)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), environment)

    assertEquals(180.dp, state.panel?.height)
    assertEquals(null, state.panel?.keyboardSpace)
    assertEquals(0.dp, effectiveImeInset(environment))
  }

  @Test
  fun software_keyboard_appearing_while_panel_is_open_closes_panel() {
    val state = EditorToolbarInputState()
    val hiddenHardware =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
      )

    state.onEnvironmentChanged(hiddenHardware)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), hiddenHardware)

    val visibleSoftware =
      hiddenHardware.copy(
        imeBottom = 320.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Software),
      )
    state.onEnvironmentChanged(visibleSoftware)

    assertEquals(null, state.panel)
    assertEquals(null, state.activeBottomPanel)
    assertEquals(320.dp, state.rememberedKeyboardInset)
    assertEquals(320.dp, effectiveImeInset(visibleSoftware))
  }

  @Test
  fun inactive_editor_does_not_remember_keyboard_inset() {
    val state = EditorToolbarInputState()
    val inactive = toolbarInputEnvironment(focused = false, imeBottom = 320.dp)

    state.onEnvironmentChanged(inactive)

    assertEquals(null, state.activeBottomPanel)
    assertEquals(320.dp, effectiveImeInset(inactive))
    assertEquals(0.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun closing_minimum_height_panel_keeps_exit_content_height() {
    val state = EditorToolbarInputState()
    val hardware =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
      )

    state.onEnvironmentChanged(hardware)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), hardware)
    state.dispatch(ToolbarIntent.RestoreEditorInput, hardware)

    assertEquals(180.dp, state.lastBottomPanelHeight)
  }
}

private fun toolbarInputEnvironment(
  visible: Boolean = true,
  focused: Boolean = true,
  imeBottom: androidx.compose.ui.unit.Dp = 0.dp,
  safeBottomInset: androidx.compose.ui.unit.Dp = 24.dp,
  keyboardState: EditorKeyboardState = EditorKeyboardState(EditorKeyboardType.Software),
  panelTransitionIdle: Boolean = true,
): ToolbarInputEnvironment =
  ToolbarInputEnvironment(
    visible = visible,
    focused = focused,
    imeBottom = imeBottom,
    safeBottomInset = safeBottomInset,
    keyboardState = keyboardState,
    panelTransitionIdle = panelTransitionIdle,
  )
