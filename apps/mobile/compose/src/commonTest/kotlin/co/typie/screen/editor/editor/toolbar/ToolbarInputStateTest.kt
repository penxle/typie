package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.state.EditorInputEffect
import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarInputStateTest {
  @Test
  fun hide_input_closes_panel_and_keyboard_without_restoring_or_clearing_focus() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), keyboardVisible)

    val effects = state.dispatch(ToolbarIntent.HideInput, keyboardVisible)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(listOf(EditorInputEffect.HideKeyboard), effects)
  }

  @Test
  fun hide_input_returns_keyboard_effect_before_toolbar_visibility_reset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), keyboardVisible)

    val effects = state.dispatch(ToolbarIntent.HideInput, keyboardVisible)
    state.onEnvironmentChanged(keyboardVisible.copy(visible = false))

    assertEquals(listOf(EditorInputEffect.HideKeyboard), effects)
  }

  @Test
  fun software_keyboard_panel_open_close_restores_keyboard_without_height_collapse() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    val openEffects =
      state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)

    assertEquals(listOf(EditorInputEffect.HideKeyboard), openEffects)
    assertEquals(
      PanelSession(
        panel = EditorToolbarBottomPanel.Insert,
        scope = EditorToolbarScope.Main,
        height = 288.dp,
        keyboardSpace = PanelKeyboardSpace.FollowIme(inset = 320.dp),
      ),
      state.panel,
    )
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    val restoreEffects = state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

    assertEquals(
      listOf(EditorInputEffect.RequestFocus, EditorInputEffect.ShowKeyboard),
      restoreEffects,
    )
    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(null, state.panel)
    assertEquals(EditorToolbarBottomPanel.Insert, state.lastBottomPanel)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), environment)

    val transitioning = environment.copy(panelTransitionRunning = true)
    val effects =
      state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), transitioning)

    assertEquals(EditorToolbarBottomPanel.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
    assertEquals(emptyList(), effects)
  }

  @Test
  fun panel_session_keeps_opening_toolbar_scope() {
    val state = EditorToolbarInputState()
    val environment = toolbarInputEnvironment(imeBottom = 320.dp)
    val blockquoteScope =
      EditorToolbarScope(pageKey = EditorToolbarPageKey.Blockquote, ownerNodeId = "blockquote-1")

    state.onEnvironmentChanged(environment)
    state.dispatch(
      ToolbarIntent.OpenPanel(
        panel =
          EditorToolbarBottomPanel.BlockquoteVariants(
            BlockquoteVariantPanelTarget.Existing(
              nodeId = "blockquote-1",
              currentVariant = co.typie.editor.ffi.BlockquoteVariant.LeftLine,
            )
          ),
        scope = blockquoteScope,
      ),
      environment,
    )

    assertEquals(blockquoteScope, state.panel?.scope)
  }

  @Test
  fun switching_panel_replaces_session_scope() {
    val state = EditorToolbarInputState()
    val environment = toolbarInputEnvironment(imeBottom = 320.dp)
    val blockquoteScope =
      EditorToolbarScope(pageKey = EditorToolbarPageKey.Blockquote, ownerNodeId = "blockquote-1")

    state.onEnvironmentChanged(environment)
    state.dispatch(
      ToolbarIntent.OpenPanel(
        panel =
          EditorToolbarBottomPanel.BlockquoteVariants(
            BlockquoteVariantPanelTarget.Existing(
              nodeId = "blockquote-1",
              currentVariant = co.typie.editor.ffi.BlockquoteVariant.LeftLine,
            )
          ),
        scope = blockquoteScope,
      ),
      environment,
    )
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), environment)

    assertEquals(EditorToolbarBottomPanel.Tools, state.activeBottomPanel)
    assertEquals(EditorToolbarScope.Main, state.panel?.scope)
  }

  @Test
  fun opening_panel_while_restoring_keyboard_reuses_restore_inset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

    val restoring = toolbarInputEnvironment(imeBottom = 120.dp, panelTransitionRunning = true)
    state.onEnvironmentChanged(restoring)
    val effects = state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), restoring)

    assertEquals(EditorToolbarBottomPanel.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
    assertEquals(listOf(EditorInputEffect.HideKeyboard), effects)
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
  fun system_ime_dismiss_while_editor_focused_clears_focus() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    val effects = state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))

    assertEquals(listOf(EditorInputEffect.ClearFocus), effects)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(0.dp, state.retainedKeyboardInset())

    val focusCleared =
      state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 0.dp))

    assertEquals(emptyList(), focusCleared)
  }

  @Test
  fun system_ime_dismiss_keeps_focus_while_panel_open() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)
    val effects = state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))

    assertEquals(EditorToolbarBottomPanel.Insert, state.activeBottomPanel)
    assertEquals(emptyList(), effects)
  }

  @Test
  fun system_ime_dismiss_during_panel_transition_does_not_clear_focus() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    val effects =
      state.onEnvironmentChanged(
        toolbarInputEnvironment(imeBottom = 0.dp, panelTransitionRunning = true)
      )

    assertEquals(emptyList(), effects)
  }

  @Test
  fun system_ime_dismiss_with_hardware_keyboard_keeps_focus() {
    val state = EditorToolbarInputState()
    val keyboardVisible =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeFrameVisible = true),
      )

    state.onEnvironmentChanged(keyboardVisible)
    val effects =
      state.onEnvironmentChanged(
        toolbarInputEnvironment(
          imeBottom = 0.dp,
          keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
        )
      )

    assertEquals(emptyList(), effects)
  }

  @Test
  fun system_ime_dismiss_keeps_focus_when_hardware_keyboard_attached_before_type_updates() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    val effects =
      state.onEnvironmentChanged(
        toolbarInputEnvironment(
          imeBottom = 0.dp,
          keyboardState =
            EditorKeyboardState(
              type = EditorKeyboardType.Software,
              hardwareKeyboardAttached = true,
            ),
        )
      )

    assertEquals(emptyList(), effects)
  }

  @Test
  fun system_ime_dismiss_during_keyboard_restore_clears_focus_and_restore() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 95.dp))

    val effects = state.onEnvironmentChanged(keyboardHidden)

    assertEquals(listOf(EditorInputEffect.ClearFocus), effects)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.rememberedKeyboardInset)
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
        activeBottomPanel = EditorToolbarBottomPanel.Tools,
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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))

    val editorFocusLost = toolbarInputEnvironment(focused = false, imeBottom = 260.dp)
    val effects = state.onEnvironmentChanged(editorFocusLost)

    assertEquals(null, state.panel)
    assertEquals(null, state.activeBottomPanel)
    assertEquals(EditorToolbarBottomPanel.Insert, state.lastBottomPanel)
    assertEquals(260.dp, state.retainedKeyboardInset())
    assertEquals(false, isEditorToolbarPresented(editorFocusLost, state.activeBottomPanel))
    assertEquals(emptyList(), effects)
  }

  @Test
  fun panel_closes_when_editor_focus_is_cleared_without_keyboard() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))

    val editorFocusCleared = toolbarInputEnvironment(focused = false, imeBottom = 0.dp)
    val effects = state.onEnvironmentChanged(editorFocusCleared)

    assertEquals(null, state.activeBottomPanel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.retainedKeyboardInset())
    assertEquals(false, isEditorToolbarPresented(editorFocusCleared, state.activeBottomPanel))
    assertEquals(emptyList(), effects)
  }

  @Test
  fun external_keyboard_after_editor_focus_loss_is_not_hidden_by_panel() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))
    state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 0.dp))

    val externalKeyboardVisible = toolbarInputEnvironment(focused = false, imeBottom = 260.dp)
    val effects = state.onEnvironmentChanged(externalKeyboardVisible)

    assertEquals(null, state.panel)
    assertEquals(null, state.activeBottomPanel)
    assertEquals(false, isEditorToolbarPresented(externalKeyboardVisible, state.activeBottomPanel))
    assertEquals(emptyList(), effects)
  }

  @Test
  fun editor_refocus_after_external_keyboard_can_open_and_restore_panel_input() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)
    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 0.dp))
    state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 0.dp))
    state.onEnvironmentChanged(toolbarInputEnvironment(focused = false, imeBottom = 320.dp))

    val editorRefocused = toolbarInputEnvironment(imeBottom = 320.dp)
    state.onEnvironmentChanged(editorRefocused)
    val openEffects =
      state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), editorRefocused)

    assertEquals(EditorToolbarBottomPanel.Insert, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(listOf(EditorInputEffect.HideKeyboard), openEffects)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    val restoreEffects = state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

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
      listOf(EditorInputEffect.RequestFocus, EditorInputEffect.ShowKeyboard),
      restoreEffects,
    )
  }

  @Test
  fun stale_keyboard_restore_after_reopening_panel_before_ime_moves_does_not_close_panel() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

    val hiddenRestoring = toolbarInputEnvironment(imeBottom = 0.dp, panelTransitionRunning = true)
    state.onEnvironmentChanged(hiddenRestoring)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), hiddenRestoring)

    val staleRestoredKeyboard = toolbarInputEnvironment(imeBottom = 320.dp)
    state.onEnvironmentChanged(staleRestoredKeyboard)

    assertEquals(EditorToolbarBottomPanel.Tools, state.activeBottomPanel)
    assertEquals(288.dp, state.panel?.height)
    assertEquals(320.dp, state.panel?.keyboardSpace?.inset)
  }

  @Test
  fun keyboard_restore_owns_layout_until_live_ime_returns_to_restore_inset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 350.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

    state.onEnvironmentChanged(toolbarInputEnvironment(imeBottom = 95.dp))

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(320.dp, state.retainedKeyboardInset())
  }

  @Test
  fun keyboard_restore_releases_when_smaller_keyboard_reaches_settled_inset() {
    val state = EditorToolbarInputState()
    val keyboardVisible = toolbarInputEnvironment(imeBottom = 320.dp)

    state.onEnvironmentChanged(keyboardVisible)
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Tools), keyboardVisible)

    val keyboardHidden = toolbarInputEnvironment(imeBottom = 0.dp)
    state.onEnvironmentChanged(keyboardHidden)
    state.dispatch(ToolbarIntent.RestoreEditorInput, keyboardHidden)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), keyboardVisible)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), software)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), software)

    val staleHardwareIme =
      toolbarInputEnvironment(
        imeBottom = 320.dp,
        keyboardState = EditorKeyboardState(EditorKeyboardType.Hardware),
      )
    state.onEnvironmentChanged(staleHardwareIme)

    assertEquals(180.dp, state.panel?.height)
    assertEquals(null, state.panel?.keyboardSpace)
    assertEquals(0.dp, effectiveImeInset(staleHardwareIme))

    val effects = state.dispatch(ToolbarIntent.RestoreEditorInput, staleHardwareIme)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.retainedKeyboardInset())
    assertEquals(listOf(EditorInputEffect.RequestFocus), effects)
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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), environment)

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
    val effects =
      state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), environment)

    assertEquals(320.dp, effectiveImeInset(environment))
    assertEquals(288.dp, state.panel?.height)
    assertEquals(
      PanelKeyboardSpace.Fixed(inset = 320.dp, restoreKeyboardOnClose = true),
      state.panel?.keyboardSpace,
    )
    assertEquals(listOf(EditorInputEffect.HideKeyboard), effects)
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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), visibleIme)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), visibleIme)

    val hiddenIme =
      toolbarInputEnvironment(
        imeBottom = 0.dp,
        keyboardState =
          EditorKeyboardState(type = EditorKeyboardType.Hardware, imeHideEventVersion = 1),
        panelTransitionRunning = true,
      )
    state.onEnvironmentChanged(hiddenIme)
    val effects = state.dispatch(ToolbarIntent.RestoreEditorInput, hiddenIme)
    state.onEnvironmentChanged(hiddenIme.copy(panelTransitionRunning = true))

    assertEquals(null, state.panel)
    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(320.dp, state.retainedKeyboardInset())
    assertEquals(listOf(EditorInputEffect.RequestFocus, EditorInputEffect.ShowKeyboard), effects)
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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), visibleIme)

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
    val effects = state.dispatch(ToolbarIntent.RestoreEditorInput, hiddenEvent)

    assertEquals(320.dp, state.keyboardRestoreInset)
    assertEquals(listOf(EditorInputEffect.RequestFocus, EditorInputEffect.ShowKeyboard), effects)
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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), visibleIme)

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
    val effects = state.dispatch(ToolbarIntent.RestoreEditorInput, userHiddenIme)

    assertEquals(null, state.panel)
    assertEquals(null, state.keyboardRestoreInset)
    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(0.dp, state.retainedKeyboardInset())
    assertEquals(listOf(EditorInputEffect.RequestFocus), effects)
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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), visibleIme)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), hiddenHardware)

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
    state.dispatch(ToolbarIntent.OpenPanel(EditorToolbarBottomPanel.Insert), hardware)
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
