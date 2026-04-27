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
  fun visible_software_keyboard_uses_software_keyboard_policy_even_when_hardware_keyboard_is_connected() {
    assertEquals(
      EditorKeyboardType.Software,
      resolveEffectiveEditorKeyboardType(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = true,
      ),
    )
  }

  @Test
  fun hidden_software_keyboard_keeps_detected_hardware_keyboard_policy() {
    assertEquals(
      EditorKeyboardType.Hardware,
      resolveEffectiveEditorKeyboardType(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun panel_opened_from_software_keyboard_restores_software_keyboard_even_when_hardware_keyboard_is_detected() {
    assertEquals(
      true,
      resolveEditorToolbarShouldRestoreSoftwareKeyboard(
        softwareKeyboardRestorePendingForPanel = true
      ),
    )
  }

  @Test
  fun panel_opened_from_software_keyboard_restores_software_keyboard_in_software_keyboard_mode() {
    assertEquals(
      true,
      resolveEditorToolbarShouldRestoreSoftwareKeyboard(
        softwareKeyboardRestorePendingForPanel = true
      ),
    )
  }

  @Test
  fun panel_opened_without_software_keyboard_does_not_restore_software_keyboard() {
    assertEquals(
      false,
      resolveEditorToolbarShouldRestoreSoftwareKeyboard(
        softwareKeyboardRestorePendingForPanel = false
      ),
    )
  }

  @Test
  fun hardware_keyboard_cancels_explicit_software_keyboard_restore() {
    assertEquals(
      false,
      resolveEditorToolbarShouldRestoreSoftwareKeyboard(
        softwareKeyboardRestorePendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
  }

  @Test
  fun restore_pending_without_actual_hardware_connection_uses_software_keyboard_policy() {
    val keyboardType =
      resolveEditorToolbarRestoreKeyboardType(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardRestorePendingForPanel = true,
        hardwareKeyboardConnected = false,
      )

    assertEquals(
      true,
      resolveEditorToolbarShouldRestoreSoftwareKeyboard(
        softwareKeyboardRestorePendingForPanel = true,
        keyboardType = keyboardType,
      ),
    )
  }

  @Test
  fun panel_input_state_suppresses_and_restores_visible_software_keyboard() {
    val inputState =
      resolveEditorToolbarPanelInputState(
        keyboardType = EditorKeyboardType.Software,
        softwareKeyboardVisible = true,
      )

    assertEquals(true, inputState.softwareKeyboardSuppressed)
    assertEquals(true, inputState.softwareKeyboardRestorePending)
    assertEquals(true, inputState.keyboardSizedPanel)
    assertEquals(true, inputState.tracksImeInset)
    assertEquals(true, inputState.hardwareKeyboardModeSwitchPending)
  }

  @Test
  fun opening_bottom_panel_hides_visible_software_keyboard() {
    assertEquals(
      true,
      shouldHideSoftwareKeyboardWhenOpeningBottomPanel(softwareKeyboardVisible = true),
    )
  }

  @Test
  fun opening_bottom_panel_does_not_hide_keyboard_without_visible_ime() {
    assertEquals(
      false,
      shouldHideSoftwareKeyboardWhenOpeningBottomPanel(softwareKeyboardVisible = false),
    )
  }

  @Test
  fun panel_input_state_suppresses_and_waits_for_hardware_switch_without_explicit_restore_in_hardware_keyboard_mode() {
    val inputState =
      resolveEditorToolbarPanelInputState(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = true,
      )

    assertEquals(true, inputState.softwareKeyboardSuppressed)
    assertEquals(false, inputState.softwareKeyboardRestorePending)
    assertEquals(true, inputState.keyboardSizedPanel)
    assertEquals(true, inputState.tracksImeInset)
    assertEquals(true, inputState.hardwareKeyboardModeSwitchPending)
  }

  @Test
  fun hardware_keyboard_mode_does_not_switch_panel_while_software_keyboard_is_visible() {
    assertEquals(
      false,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = true,
        tracksImeInsetForPanel = true,
        softwareKeyboardRestorePendingForPanel = true,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 0,
        hardwareKeyboardModeSwitchPendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun pending_hardware_keyboard_mode_switches_panel_after_ime_hides() {
    assertEquals(
      true,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = true,
        softwareKeyboardRestorePendingForPanel = true,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 0,
        hardwareKeyboardModeSwitchPendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_switches_panel_when_hardware_state_is_observed_after_ime_hides() {
    assertEquals(
      true,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = true,
        softwareKeyboardRestorePendingForPanel = true,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 0,
        hardwareKeyboardModeSwitchPendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_does_not_switch_panel_opened_from_visible_software_keyboard_when_hardware_was_already_detected() {
    assertEquals(
      false,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = true,
        softwareKeyboardRestorePendingForPanel = false,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 0,
        hardwareKeyboardModeSwitchPendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_switches_panel_when_hardware_connects_after_panel_opens() {
    assertEquals(
      true,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = true,
        softwareKeyboardRestorePendingForPanel = false,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 1,
        hardwareKeyboardModeSwitchPendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_does_not_switch_panel_without_actual_hardware_connection() {
    assertEquals(
      false,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = true,
        softwareKeyboardRestorePendingForPanel = true,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 1,
        hardwareKeyboardModeSwitchPendingForPanel = true,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = false,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_does_not_switch_panel_opened_without_visible_software_keyboard() {
    assertEquals(
      false,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = false,
        softwareKeyboardRestorePendingForPanel = false,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 0,
        hardwareKeyboardModeSwitchPendingForPanel = false,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_does_not_switch_desktop_panel_without_software_to_hardware_transition() {
    assertEquals(
      false,
      shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        tracksImeInsetForPanel = false,
        softwareKeyboardRestorePendingForPanel = false,
        hardwareKeyboardModeGenerationAtOpen = 0,
        hardwareKeyboardModeGeneration = 0,
        hardwareKeyboardModeSwitchPendingForPanel = false,
        keyboardType = EditorKeyboardType.Hardware,
        hardwareKeyboardConnected = true,
      ),
    )
  }

  @Test
  fun editor_keyboard_state_derives_connection_from_type() {
    assertEquals(
      true,
      EditorKeyboardState(type = EditorKeyboardType.Hardware).hardwareKeyboardConnected,
    )
    assertEquals(
      false,
      EditorKeyboardState(type = EditorKeyboardType.Software).hardwareKeyboardConnected,
    )
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
  fun software_keyboard_without_visible_keyboard_uses_fallback_reserved_inset() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(252.dp, state.visibleImeInset(imeBottom = 0.dp, safeBottomInset = 24.dp))
    assertEquals(220.dp, state.bottomPanelHeight(imeBottom = 0.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun hardware_keyboard_bottom_panel_uses_minimum_height_when_opened_directly() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
      keyboardType = EditorKeyboardType.Hardware,
    )

    assertEquals(
      212.dp,
      state.visibleImeInset(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
    assertEquals(
      180.dp,
      state.bottomPanelHeight(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
  }

  @Test
  fun hardware_keyboard_bottom_panel_keeps_minimum_height_when_reopened() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.switchOpenPanelToHardwareKeyboardMode()
    state.closePanel()
    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
      keyboardType = EditorKeyboardType.Hardware,
    )

    assertEquals(
      180.dp,
      state.bottomPanelHeight(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
  }

  @Test
  fun software_keyboard_bottom_panel_uses_minimum_height_when_keyboard_is_short() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 100.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(212.dp, state.visibleImeInset(imeBottom = 100.dp, safeBottomInset = 24.dp))
    assertEquals(180.dp, state.bottomPanelHeight(imeBottom = 100.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun software_keyboard_bottom_panel_uses_keyboard_height_when_keyboard_is_tall() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(320.dp, state.visibleImeInset(imeBottom = 320.dp, safeBottomInset = 24.dp))
    assertEquals(288.dp, state.bottomPanelHeight(imeBottom = 320.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun open_bottom_panel_height_change_animates_when_software_keyboard_is_not_visible() {
    assertEquals(
      true,
      shouldAnimateEditorToolbarBottomPanelLayoutHeightChange(
        bottomPanelVisible = true,
        softwareKeyboardVisible = false,
        previousBottomPanelLayoutHeight = 320.dp,
        bottomPanelLayoutHeight = 212.dp,
      ),
    )
  }

  @Test
  fun open_bottom_panel_height_change_does_not_animate_while_tracking_software_keyboard() {
    assertEquals(
      false,
      shouldAnimateEditorToolbarBottomPanelLayoutHeightChange(
        bottomPanelVisible = true,
        softwareKeyboardVisible = true,
        previousBottomPanelLayoutHeight = 212.dp,
        bottomPanelLayoutHeight = 320.dp,
      ),
    )
  }

  @Test
  fun hidden_bottom_panel_height_change_does_not_animate_as_resize() {
    assertEquals(
      false,
      shouldAnimateEditorToolbarBottomPanelLayoutHeightChange(
        bottomPanelVisible = false,
        softwareKeyboardVisible = false,
        previousBottomPanelLayoutHeight = 212.dp,
        bottomPanelLayoutHeight = 0.dp,
      ),
    )
  }

  @Test
  fun software_keyboard_bottom_panel_keeps_opening_keyboard_height_while_keyboard_hides() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 40.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(320.dp, state.rememberedKeyboardInset)
    assertEquals(320.dp, state.visibleImeInset(imeBottom = 40.dp, safeBottomInset = 24.dp))
    assertEquals(288.dp, state.bottomPanelHeight(imeBottom = 40.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun software_keyboard_bottom_panel_uses_new_keyboard_height_when_reopened_with_smaller_keyboard() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 40.dp,
      safeBottomInset = 24.dp,
    )
    state.closePanel()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 260.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(260.dp, state.rememberedKeyboardInset)
    assertEquals(260.dp, state.visibleImeInset(imeBottom = 260.dp, safeBottomInset = 24.dp))
    assertEquals(228.dp, state.bottomPanelHeight(imeBottom = 260.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun closing_software_keyboard_panel_keeps_opening_height_while_keyboard_restores() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.closePanel(keepRememberedKeyboardInsetUntilImeRestored = true)
    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 95.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(320.dp, state.rememberedKeyboardInset)
    assertEquals(320.dp, state.visibleImeInset(imeBottom = 95.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun restored_software_keyboard_clears_locked_opening_height() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.closePanel(keepRememberedKeyboardInsetUntilImeRestored = true)
    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.clearRememberedKeyboardInsetIfRestored(imeBottom = 320.dp)

    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(320.dp, state.visibleImeInset(imeBottom = 320.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun open_bottom_panel_keeps_text_input_session_enabled_for_hardware_keyboard() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun open_bottom_panel_keeps_text_input_session_enabled_when_software_keyboard_is_suppressed() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(true, state.softwareKeyboardSuppressedForPanel)
    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = true,
      ),
    )
  }

  @Test
  fun bottom_panel_opened_from_software_keyboard_keeps_text_input_session_enabled_after_keyboard_hides() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Software,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun bottom_panel_opened_from_software_keyboard_tracks_restore_pending() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(true, state.softwareKeyboardRestorePendingForPanel)
  }

  @Test
  fun bottom_panel_opened_from_visible_software_keyboard_with_hardware_keyboard_does_not_restore() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
      keyboardType = EditorKeyboardType.Hardware,
    )

    assertEquals(true, state.softwareKeyboardSuppressedForPanel)
    assertEquals(false, state.softwareKeyboardRestorePendingForPanel)
    assertEquals(true, state.hardwareKeyboardModeSwitchPendingForPanel)
    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun hardware_sized_switch_for_panel_opened_from_visible_software_keyboard_in_hardware_mode_keeps_text_input_suppressed() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
      keyboardType = EditorKeyboardType.Hardware,
    )
    state.switchOpenPanelToHardwareKeyboardMode()

    assertEquals(true, state.softwareKeyboardSuppressedForPanel)
    assertEquals(false, state.softwareKeyboardRestorePendingForPanel)
    assertEquals(true, state.keyboardSizedPanel)
    assertEquals(false, state.tracksImeInsetForPanel)
    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(
      212.dp,
      state.visibleImeInset(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
    assertEquals(
      180.dp,
      state.bottomPanelHeight(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun close_keeps_remembered_inset_for_implicit_software_keyboard_restore() {
    val policy =
      resolveEditorToolbarRememberedKeyboardInsetClosePolicy(
        shouldRestoreSoftwareKeyboard = false,
        softwareKeyboardSuppressedForPanel = true,
        softwareKeyboardRestorePendingForPanel = false,
      )

    assertEquals(
      EditorToolbarRememberedKeyboardInsetClosePolicy.KeepForImplicitSoftwareKeyboardRestore,
      policy,
    )
    assertEquals(false, policy.clearBeforePanelClose)
    assertEquals(true, policy.restoreFallbackAfterPanelClose)
  }

  @Test
  fun close_keeps_remembered_inset_for_explicit_software_keyboard_restore() {
    val policy =
      resolveEditorToolbarRememberedKeyboardInsetClosePolicy(
        shouldRestoreSoftwareKeyboard = true,
        softwareKeyboardSuppressedForPanel = true,
        softwareKeyboardRestorePendingForPanel = true,
      )

    assertEquals(
      EditorToolbarRememberedKeyboardInsetClosePolicy.KeepForExplicitSoftwareKeyboardRestore,
      policy,
    )
    assertEquals(false, policy.clearBeforePanelClose)
    assertEquals(false, policy.restoreFallbackAfterPanelClose)
  }

  @Test
  fun close_clears_remembered_inset_when_panel_did_not_suppress_keyboard() {
    val policy =
      resolveEditorToolbarRememberedKeyboardInsetClosePolicy(
        shouldRestoreSoftwareKeyboard = false,
        softwareKeyboardSuppressedForPanel = false,
        softwareKeyboardRestorePendingForPanel = false,
      )

    assertEquals(EditorToolbarRememberedKeyboardInsetClosePolicy.ClearBeforePanelClose, policy)
    assertEquals(true, policy.clearBeforePanelClose)
    assertEquals(false, policy.restoreFallbackAfterPanelClose)
  }

  @Test
  fun close_clears_remembered_inset_when_hardware_keyboard_cancels_explicit_software_restore() {
    val policy =
      resolveEditorToolbarRememberedKeyboardInsetClosePolicy(
        shouldRestoreSoftwareKeyboard = false,
        softwareKeyboardSuppressedForPanel = true,
        softwareKeyboardRestorePendingForPanel = true,
      )

    assertEquals(EditorToolbarRememberedKeyboardInsetClosePolicy.ClearBeforePanelClose, policy)
    assertEquals(true, policy.clearBeforePanelClose)
    assertEquals(false, policy.restoreFallbackAfterPanelClose)
  }

  @Test
  fun hardware_keyboard_after_explicit_restore_clears_remembered_keyboard_inset() {
    assertEquals(
      true,
      shouldClearRememberedKeyboardInsetAfterHardwareKeyboardRestore(
        softwareKeyboardRestorePending = true,
        bottomPanelVisible = false,
        softwareKeyboardVisible = false,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
  }

  @Test
  fun explicit_restore_keeps_remembered_keyboard_inset_while_waiting_for_software_keyboard() {
    assertEquals(
      false,
      shouldClearRememberedKeyboardInsetAfterHardwareKeyboardRestore(
        softwareKeyboardRestorePending = true,
        bottomPanelVisible = false,
        softwareKeyboardVisible = false,
        keyboardType = EditorKeyboardType.Software,
      ),
    )
  }

  @Test
  fun switching_open_panel_to_hardware_keyboard_mode_clears_restore_and_remembered_inset() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.switchOpenPanelToHardwareKeyboardMode()

    assertEquals(false, state.softwareKeyboardSuppressedForPanel)
    assertEquals(false, state.softwareKeyboardRestorePendingForPanel)
    assertEquals(true, state.keyboardSizedPanel)
    assertEquals(false, state.tracksImeInsetForPanel)
    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(
      212.dp,
      state.visibleImeInset(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
    assertEquals(
      180.dp,
      state.bottomPanelHeight(
        imeBottom = 0.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
    assertEquals(
      212.dp,
      state.visibleImeInset(
        imeBottom = 260.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
    assertEquals(
      180.dp,
      state.bottomPanelHeight(
        imeBottom = 260.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )
  }

  @Test
  fun hardware_keyboard_mode_ignores_stale_ime_after_panel_close_until_ime_hides() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.switchOpenPanelToHardwareKeyboardMode()
    state.closePanel()

    assertEquals(
      24.dp,
      state.inputBottomInset(
        imeBottom = 320.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )

    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Hardware,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(320.dp, state.inputBottomInset(imeBottom = 320.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun software_keyboard_mode_after_hardware_disconnect_uses_software_keyboard_inset() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.switchOpenPanelToHardwareKeyboardMode()
    state.closePanel()

    assertEquals(
      24.dp,
      state.inputBottomInset(
        imeBottom = 320.dp,
        safeBottomInset = 24.dp,
        keyboardType = EditorKeyboardType.Hardware,
      ),
    )

    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(320.dp, state.inputBottomInset(imeBottom = 320.dp, safeBottomInset = 24.dp))
    assertEquals(320.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun closing_bottom_panel_allows_text_input_session_again() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.closePanel()

    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun clearing_remembered_keyboard_inset_after_panel_close_restores_safe_input_bottom_inset() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.closePanel()
    state.clearRememberedKeyboardInset()

    assertEquals(24.dp, state.inputBottomInset(imeBottom = 0.dp, safeBottomInset = 24.dp))
  }

  @Test
  fun inactive_editor_does_not_remember_visible_software_keyboard_inset() {
    val state = EditorToolbarBottomState()

    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
      editorInputActive = false,
    )

    assertEquals(0.dp, state.rememberedKeyboardInset)
    assertEquals(false, state.toolbarVisible(visible = true, editorFocused = false))
  }

  @Test
  fun inactive_editor_remembers_software_keyboard_again_when_input_becomes_active() {
    val state = EditorToolbarBottomState()

    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
      editorInputActive = false,
    )
    state.syncKeyboardEnvironment(
      keyboardType = EditorKeyboardType.Software,
      imeBottom = 260.dp,
      safeBottomInset = 24.dp,
      editorInputActive = true,
    )

    assertEquals(260.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun software_keyboard_appearing_while_bottom_panel_is_open_keeps_text_input_session_enabled() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Hardware,
        softwareKeyboardVisible = true,
        softwareKeyboardAppearing = true,
      ),
    )
  }

  @Test
  fun software_keyboard_appearing_while_hardware_sized_panel_is_open_closes_panel() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.switchOpenPanelToHardwareKeyboardMode()
    state.syncOpenPanelWithSoftwareKeyboardAppearance(
      previousSoftwareKeyboardVisible = false,
      softwareKeyboardVisible = true,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(null, state.activePanel)
    assertEquals(320.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun software_keyboard_appearing_while_fixed_bottom_panel_is_open_closes_panel() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
    )
    state.syncOpenPanelWithSoftwareKeyboardAppearance(
      previousSoftwareKeyboardVisible = false,
      softwareKeyboardVisible = true,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(null, state.activePanel)
    assertEquals(320.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun already_visible_software_keyboard_does_not_close_newly_opened_bottom_panel() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )
    state.syncOpenPanelWithSoftwareKeyboardAppearance(
      previousSoftwareKeyboardVisible = true,
      softwareKeyboardVisible = true,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(EditorToolbarBottomPanelKey.Insert, state.activePanel)
  }

  @Test
  fun software_keyboard_appearing_closes_open_bottom_panel() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 0.dp,
      safeBottomInset = 24.dp,
      keyboardType = EditorKeyboardType.Hardware,
    )
    state.syncOpenPanelWithSoftwareKeyboardAppearance(
      previousSoftwareKeyboardVisible = false,
      softwareKeyboardVisible = true,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(null, state.activePanel)
    assertEquals(320.dp, state.rememberedKeyboardInset)
  }

  @Test
  fun software_keyboard_appearance_requests_open_bottom_panel_close() {
    assertEquals(
      true,
      shouldCloseOpenEditorToolbarPanelWhenSoftwareKeyboardAppears(
        bottomPanelVisible = true,
        previousSoftwareKeyboardVisible = false,
        softwareKeyboardVisible = true,
      ),
    )
  }

  @Test
  fun already_visible_software_keyboard_does_not_request_bottom_panel_close() {
    assertEquals(
      false,
      shouldCloseOpenEditorToolbarPanelWhenSoftwareKeyboardAppears(
        bottomPanelVisible = true,
        previousSoftwareKeyboardVisible = true,
        softwareKeyboardVisible = true,
      ),
    )
  }

  @Test
  fun open_bottom_panel_keeps_text_input_session_enabled_for_suppressed_software_keyboard() {
    val state = EditorToolbarBottomState()

    state.openPanel(
      panel = EditorToolbarBottomPanelKey.Insert,
      imeBottom = 320.dp,
      safeBottomInset = 24.dp,
    )

    assertEquals(
      true,
      state.textInputSessionEnabled(
        keyboardType = EditorKeyboardType.Software,
        softwareKeyboardVisible = true,
      ),
    )
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

  @Test
  fun visible_bottom_panel_spacer_height_includes_panel_and_safe_inset() {
    assertEquals(
      252.dp,
      resolveEditorToolbarBottomSpacerHeight(
        bottomPanelVisible = true,
        bottomPanelLayoutHeight = 228.dp,
        inputBottomInset = 320.dp,
        safeBottomInset = 24.dp,
      ),
    )
  }

  @Test
  fun hidden_bottom_panel_spacer_height_uses_input_inset() {
    assertEquals(
      320.dp,
      resolveEditorToolbarBottomSpacerHeight(
        bottomPanelVisible = false,
        bottomPanelLayoutHeight = 228.dp,
        inputBottomInset = 320.dp,
        safeBottomInset = 24.dp,
      ),
    )
  }

  @Test
  fun toolbar_bottom_inset_subtracts_visible_panel_height_from_total_space() {
    assertEquals(
      72.dp,
      resolveEditorToolbarBottomInset(
        bottomSpacerHeight = 300.dp,
        bottomPanelLayoutHeight = 228.dp,
        safeBottomInset = 24.dp,
      ),
    )
  }

  @Test
  fun toolbar_bottom_inset_keeps_safe_inset_while_panel_is_collapsing() {
    assertEquals(
      24.dp,
      resolveEditorToolbarBottomInset(
        bottomSpacerHeight = 80.dp,
        bottomPanelLayoutHeight = 228.dp,
        safeBottomInset = 24.dp,
      ),
    )
  }
}
