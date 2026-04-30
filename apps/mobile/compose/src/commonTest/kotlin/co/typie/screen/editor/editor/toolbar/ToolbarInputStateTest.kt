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
        keyboardSpace = PanelKeyboardSpace.FollowIme(inset = 320.dp),
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
    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(null, state.panel)
    assertEquals(EditorToolbarBottomPanelKey.Insert, state.lastBottomPanel)

    val hiddenRestoring = keyboardHidden.copy(panelTransitionRunning = true)
    state.onEnvironmentChanged(hiddenRestoring)

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(320.dp, state.retainedKeyboardInset())

    val restoring = toolbarInputEnvironment(imeBottom = 95.dp, panelTransitionRunning = true)
    state.onEnvironmentChanged(restoring)

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(288.dp, state.lastBottomPanelHeight)

    val restored = toolbarInputEnvironment(imeBottom = 320.dp)
    state.onEnvironmentChanged(restored)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(320.dp, effectiveImeInset(restored))
  }

  @Test
  fun rapid_toggle_switches_panel_while_panel_transition_is_running() {
    val state = EditorToolbarInputState()
    val environment = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(environment)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), environment)
    state.takeEffects()

    val transitioning = environment.copy(panelTransitionRunning = true)
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

    val restoring = toolbarInputEnvironment(imeBottom = 120.dp, panelTransitionRunning = true)
    state.onEnvironmentChanged(restoring)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), restoring)

    assertEquals(EditorToolbarBottomPanelKey.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
    assertEquals(listOf(ToolbarEffect.HideKeyboard), state.takeEffects())
  }

  @Test
  fun focus_loss_tracks_keyboard_inset_down_instead_of_retaining_peak() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)

    val partiallyHidden = toolbarInputEnvironment(focused = false, imeBottom = 180.dp)
    state.onEnvironmentChanged(partiallyHidden)

    assertEquals(180.dp, state.retainedKeyboardInset())

    val keyboardHidden = toolbarInputEnvironment(focused = false, imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)

    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(0.dp, state.retainedKeyboardInset())
  }

  @Test
  fun retained_keyboard_inset_does_not_present_toolbar_without_editor_focus() {
    val environment = toolbarInputEnvironment(focused = false, imeBottom = 320.dp)

    val presented = isEditorToolbarPresented(environment = environment, activeBottomPanel = null)

    assertEquals(false, presented)
  }

  @Test
  fun software_keyboard_toolbar_is_presented_when_keyboard_starts_visible_show_transition() {
    val keyboardStarting =
      toolbarInputEnvironment(
        focused = true,
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Showing,
          ),
      )

    assertEquals(
      false,
      isEditorToolbarPresented(environment = keyboardStarting, activeBottomPanel = null),
    )

    val keyboardMoving = keyboardStarting.copy(imeBottom = 64.dp)

    assertEquals(
      true,
      isEditorToolbarPresented(environment = keyboardMoving, activeBottomPanel = null),
    )
  }

  @Test
  fun software_keyboard_toolbar_exits_while_keyboard_is_hiding() {
    val environment =
      toolbarInputEnvironment(
        focused = false,
        imeBottom = 160.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Hiding,
          ),
      )

    assertEquals(
      false,
      isEditorToolbarPresented(environment = environment, activeBottomPanel = null),
    )
    assertEquals(160.dp, effectiveImeInset(environment))
  }

  @Test
  fun panel_and_restore_present_toolbar_independent_of_keyboard_phase() {
    val keyboardHidden =
      toolbarInputEnvironment(
        focused = true,
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Hidden,
          ),
      )

    assertEquals(
      true,
      isEditorToolbarPresented(
        environment = keyboardHidden,
        activeBottomPanel = EditorToolbarBottomPanelKey.Tools,
      ),
    )
    assertEquals(
      true,
      isEditorToolbarPresented(
        environment = keyboardHidden,
        activeBottomPanel = null,
        restoringEditorInput = true,
      ),
    )
  }

  @Test
  fun hardware_keyboard_toolbar_is_presented_immediately_on_editor_focus() {
    val environment =
      toolbarInputEnvironment(
        focused = true,
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Hardware,
            presentation = EditorKeyboardPresentation.Hidden,
          ),
      )

    assertEquals(
      true,
      isEditorToolbarPresented(environment = environment, activeBottomPanel = null),
    )
  }

  @Test
  fun panel_closes_when_external_keyboard_takes_focus() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))

    val editorFocusLost = toolbarInputEnvironment(focused = false, imeBottom = 260.dp)
    state.onEnvironmentChanged(editorFocusLost)

    assertEquals(null, state.panel)
    assertEquals(null, state.activeBottomPanel)
    assertEquals(EditorToolbarBottomPanelKey.Insert, state.lastBottomPanel)
    assertEquals(260.dp, state.retainedKeyboardInset())
    assertEquals(false, isEditorToolbarPresented(editorFocusLost, state.activeBottomPanel))
    assertEquals(emptyList(), state.takeEffects())
  }

  @Test
  fun panel_closes_when_editor_focus_is_cleared_without_keyboard() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))

    val editorFocusCleared = toolbarInputEnvironment(focused = false, imeBottom = 0.dp)
    state.onEnvironmentChanged(editorFocusCleared)

    assertEquals(null, state.activeBottomPanel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.retainedKeyboardInset())
    assertEquals(false, isEditorToolbarPresented(editorFocusCleared, state.activeBottomPanel))
    assertEquals(emptyList(), state.takeEffects())
  }

  @Test
  fun external_keyboard_after_editor_focus_loss_is_not_hidden_by_panel() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))
    state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 0.dp))

    val externalKeyboardVisible = toolbarInputEnvironment(focused = false, imeBottom = 260.dp)
    state.onEnvironmentChanged(externalKeyboardVisible)

    assertEquals(null, state.panel)
    assertEquals(null, state.activeBottomPanel)
    assertEquals(false, isEditorToolbarPresented(externalKeyboardVisible, state.activeBottomPanel))
    assertEquals(emptyList(), state.takeEffects())
  }

  @Test
  fun editor_refocus_after_external_keyboard_can_open_and_restore_panel_input() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))
    state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 0.dp))
    state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 320.dp))

    val editorRefocused = toolbarInputEnvironment(imeBottom = 320.dp)
    state.onEnvironmentChanged(editorRefocused)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), editorRefocused)

    assertEquals(EditorToolbarBottomPanelKey.Insert, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(listOf(ToolbarEffect.HideKeyboard), state.takeEffects())

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

    assertEquals(null, state.activeBottomPanel)
    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(
      true,
      isEditorToolbarPresented(
        environment = keyboardHidden,
        activeBottomPanel = state.activeBottomPanel,
        restoringEditorInput = state.keyboardRestoreInset != null,
      ),
    )
    assertEquals(
      listOf(ToolbarEffect.RequestFocus, ToolbarEffect.ShowKeyboard),
      state.takeEffects(),
    )
  }

  @Test
  fun stale_keyboard_restore_after_reopening_panel_before_ime_moves_does_not_close_panel() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.takeEffects()

    val hiddenRestoring = toolbarInputEnvironment(imeBottom = 0.dp, panelTransitionRunning = true)
    state.onEnvironmentChanged(hiddenRestoring)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), hiddenRestoring)
    state.takeEffects()

    val staleRestoredKeyboard = toolbarInputEnvironment(imeBottom = 320.dp)
    state.onEnvironmentChanged(staleRestoredKeyboard)

    assertEquals(EditorToolbarBottomPanelKey.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
  }

  @Test
  fun keyboard_restore_owns_layout_until_live_ime_returns_to_restore_inset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 350.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), keyboardVisible)
    state.takeEffects()

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.takeEffects()

    state.onEnvironmentChanged(
      toolbarInputEnvironment(imeBottom = 806.dp, panelTransitionRunning = true)
    )

    assertEquals(350.dp, state.keyboardRestoreInset)
    assertEquals(350.dp, state.retainedKeyboardInset())

    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 782.dp))

    assertEquals(350.dp, state.keyboardRestoreInset)
    assertEquals(350.dp, state.retainedKeyboardInset())

    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 650.dp))

    assertEquals(350.dp, state.keyboardRestoreInset)
    assertEquals(350.dp, state.retainedKeyboardInset())

    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 350.dp))

    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(350.dp, state.rememberedKeyboardInset)
    assertEquals(350.dp, state.retainedKeyboardInset())
  }

  @Test
  fun keyboard_restore_keeps_panel_space_while_live_ime_is_still_replacing_it() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), keyboardVisible)
    state.takeEffects()

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.takeEffects()

    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 95.dp))

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(320.dp, state.retainedKeyboardInset())
  }

  @Test
  fun keyboard_restore_releases_when_smaller_keyboard_reaches_settled_inset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), keyboardVisible)
    state.takeEffects()

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.takeEffects()

    val smallerKeyboardState =
      EditorKeyboardState(
        type = EditorKeyboardType.Software,
        presentation = EditorKeyboardPresentation.Shown(settledImeBottom = 280.dp),
      )
    state.onEnvironmentChanged(
      toolbarInputEnvironment(imeBottom = 160.dp, keyboardState = smallerKeyboardState)
    )

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(320.dp, state.retainedKeyboardInset())

    state.onEnvironmentChanged(
      toolbarInputEnvironment(imeBottom = 280.dp, keyboardState = smallerKeyboardState)
    )

    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(280.dp, state.retainedKeyboardInset())
  }

  @Test
  fun settled_ime_inset_bounds_ios_refocus_overshoot() {
    val environment =
      toolbarInputEnvironment(
        imeBottom = 806.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Shown(settledImeBottom = 350.dp),
          ),
      )

    assertEquals(350.dp, effectiveImeInset(environment))
  }

  @Test
  fun live_ime_inset_is_preserved_while_smaller_keyboard_is_still_appearing() {
    val environment =
      toolbarInputEnvironment(
        imeBottom = 160.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Shown(settledImeBottom = 280.dp),
          ),
      )

    assertEquals(160.dp, effectiveImeInset(environment))
  }

  @Test
  fun keyboard_restore_releases_to_settled_inset_without_remembering_refocus_overshoot() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 350.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Tools), keyboardVisible)
    state.takeEffects()

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.takeEffects()

    state.onEnvironmentChanged(
      toolbarInputEnvironment(
        imeBottom = 806.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Shown(settledImeBottom = 350.dp),
          ),
      )
    )

    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(350.dp, state.retainedKeyboardInset())
  }

  @Test
  fun restoring_editor_input_keeps_toolbar_visible_during_transient_focus_loss() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), keyboardVisible)
    state.takeEffects()

    val hiddenKeyboardDuringPanel = toolbarInputEnvironment(focused = false, imeBottom = 0.dp)
    state.dispatch(ToolbarIntent.RestoreEditorInput, hiddenKeyboardDuringPanel)

    assertEquals(null, state.activeBottomPanel)
    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(
      true,
      isEditorToolbarPresented(
        environment = hiddenKeyboardDuringPanel,
        activeBottomPanel = state.activeBottomPanel,
        restoringEditorInput = state.keyboardRestoreInset != null,
      ),
    )
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
    assertEquals(null, state.keyboardRestoreInset)
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
  fun hardware_keyboard_with_visible_ime_frame_uses_keyboard_space() {
    val state = EditorToolbarInputState()
    val environment =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(environment)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), environment)

    assertEquals(320.dp, effectiveImeInset(environment))
    assertEquals(288.dp, state.panel?.height)
    assertEquals(
      PanelKeyboardSpace.Fixed(inset = 320.dp, restoreKeyboardOnClose = true),
      state.panel?.keyboardSpace,
    )
    assertEquals(listOf(ToolbarEffect.HideKeyboard), state.takeEffects())
  }

  @Test
  fun hardware_keyboard_panel_keeps_keyboard_space_after_hide() {
    val state = EditorToolbarInputState()
    val visibleIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(visibleIme)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), visibleIme)
    state.takeEffects()

    val hiddenIme =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeHideEventVersion = 1),
        panelTransitionRunning = true,
      )
    state.onEnvironmentChanged(hiddenIme)

    assertEquals(288.dp, state.panel?.height)
    assertEquals(
      PanelKeyboardSpace.Fixed(inset = 320.dp, restoreKeyboardOnClose = true),
      state.panel?.keyboardSpace,
    )
    assertEquals(true, suppressSoftwareKeyboard(requireNotNull(state.panel)))
  }

  @Test
  fun closing_hardware_keyboard_space_panel_after_panel_hides_ime_restores_keyboard() {
    val state = EditorToolbarInputState()
    val visibleIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(visibleIme)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), visibleIme)
    state.takeEffects()

    val hiddenIme =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeHideEventVersion = 1),
        panelTransitionRunning = true,
      )
    state.onEnvironmentChanged(hiddenIme)
    state.dispatch(ToolbarIntent.RestoreEditorInput, hiddenIme)
    state.onEnvironmentChanged(hiddenIme.copy(panelTransitionRunning = true))

    assertEquals(null, state.panel)
    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(320.dp, state.retainedKeyboardInset())
    assertEquals(
      listOf(ToolbarEffect.RequestFocus, ToolbarEffect.ShowKeyboard),
      state.takeEffects(),
    )
  }

  @Test
  fun duplicated_panel_hide_event_during_transition_still_restores_keyboard() {
    val state = EditorToolbarInputState()
    val visibleIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(visibleIme)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), visibleIme)
    state.takeEffects()

    val hiddenFrame =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState = EditorKeyboardState(type = EditorKeyboardType.Hardware),
        panelTransitionRunning = true,
      )
    state.onEnvironmentChanged(hiddenFrame)

    val hiddenEvent =
      hiddenFrame.copy(
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeHideEventVersion = 1)
      )
    state.onEnvironmentChanged(hiddenEvent)
    state.dispatch(ToolbarIntent.RestoreEditorInput, hiddenEvent)

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(
      listOf(ToolbarEffect.RequestFocus, ToolbarEffect.ShowKeyboard),
      state.takeEffects(),
    )
  }

  @Test
  fun closing_hardware_keyboard_space_panel_after_user_hides_ime_does_not_restore_keyboard() {
    val state = EditorToolbarInputState()
    val visibleIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(visibleIme)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), visibleIme)
    state.takeEffects()

    val panelHiddenIme =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeHideEventVersion = 1),
        panelTransitionRunning = true,
      )
    state.onEnvironmentChanged(panelHiddenIme)
    state.onEnvironmentChanged(panelHiddenIme.copy(panelTransitionRunning = false))

    val userHiddenIme =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeHideEventVersion = 2),
      )
    state.onEnvironmentChanged(userHiddenIme)
    state.dispatch(ToolbarIntent.RestoreEditorInput, userHiddenIme)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(0.dp, state.retainedKeyboardInset())
    assertEquals(listOf(ToolbarEffect.RequestFocus), state.takeEffects())
  }

  @Test
  fun hardware_keyboard_panel_closes_when_software_keyboard_reappears() {
    val state = EditorToolbarInputState()
    val visibleIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(visibleIme)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanelKey.Insert), visibleIme)
    state.takeEffects()

    val hiddenIme =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState = EditorKeyboardState(type = EditorKeyboardType.Hardware),
      )
    state.onEnvironmentChanged(hiddenIme)
    state.onEnvironmentChanged(visibleIme)

    assertEquals(null, state.panel)
    assertEquals(320.dp, state.rememberedKeyboardInset)
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
  panelTransitionRunning: Boolean = false,
): ToolbarInputEnvironment =
  ToolbarInputEnvironment(
    visible = visible,
    focused = focused,
    imeBottom = imeBottom,
    safeBottomInset = safeBottomInset,
    keyboardState = keyboardState,
    panelTransitionRunning = panelTransitionRunning,
  )
