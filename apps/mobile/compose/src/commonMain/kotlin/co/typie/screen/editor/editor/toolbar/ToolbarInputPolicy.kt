package co.typie.screen.editor.editor.toolbar

internal enum class EditorToolbarFixedAction {
  ClosePanel,
  HideToolbar,
  DismissInput,
}

internal enum class EditorToolbarPanelHeightMode {
  Fixed,
  Keyboard,
  Minimum,
}

internal enum class EditorToolbarImeInsetMode {
  Track,
  IgnoreWhileHardwareKeyboard,
}

internal data class EditorToolbarPanelInputState(
  val softwareKeyboardSuppressed: Boolean = false,
  val softwareKeyboardRestorePending: Boolean = false,
  val heightMode: EditorToolbarPanelHeightMode = EditorToolbarPanelHeightMode.Fixed,
  val hardwareKeyboardModeSwitchPending: Boolean = false,
  val hardwareKeyboardModeGenerationAtOpen: Int = 0,
) {
  val keyboardSizedPanel: Boolean
    get() = heightMode != EditorToolbarPanelHeightMode.Fixed

  val tracksImeInset: Boolean
    get() = heightMode == EditorToolbarPanelHeightMode.Keyboard
}

internal enum class EditorToolbarRememberedKeyboardInsetClosePolicy(
  val clearBeforePanelClose: Boolean,
  val restoreFallbackAfterPanelClose: Boolean,
) {
  ClearBeforePanelClose(clearBeforePanelClose = true, restoreFallbackAfterPanelClose = false),
  KeepForExplicitSoftwareKeyboardRestore(
    clearBeforePanelClose = false,
    restoreFallbackAfterPanelClose = false,
  ),
  KeepForImplicitSoftwareKeyboardRestore(
    clearBeforePanelClose = false,
    restoreFallbackAfterPanelClose = true,
  ),
}

internal fun resolveEffectiveEditorKeyboardType(
  keyboardType: EditorKeyboardType,
  softwareKeyboardVisible: Boolean,
): EditorKeyboardType =
  if (softwareKeyboardVisible) {
    EditorKeyboardType.Software
  } else {
    keyboardType
  }

internal fun resolveEditorToolbarFixedAction(
  activeBottomPanel: EditorToolbarBottomPanelKey?,
  keyboardType: EditorKeyboardType,
  softwareKeyboardVisible: Boolean,
): EditorToolbarFixedAction =
  when {
    activeBottomPanel != null -> EditorToolbarFixedAction.ClosePanel
    keyboardType == EditorKeyboardType.Hardware && !softwareKeyboardVisible ->
      EditorToolbarFixedAction.HideToolbar
    else -> EditorToolbarFixedAction.DismissInput
  }

internal fun resolveEditorToolbarPanelInputState(
  keyboardType: EditorKeyboardType,
  softwareKeyboardVisible: Boolean,
  hardwareKeyboardModeGeneration: Int = 0,
): EditorToolbarPanelInputState =
  EditorToolbarPanelInputState(
    softwareKeyboardSuppressed = softwareKeyboardVisible,
    softwareKeyboardRestorePending =
      softwareKeyboardVisible && keyboardType == EditorKeyboardType.Software,
    heightMode =
      if (softwareKeyboardVisible) {
        EditorToolbarPanelHeightMode.Keyboard
      } else if (keyboardType == EditorKeyboardType.Hardware) {
        EditorToolbarPanelHeightMode.Minimum
      } else {
        EditorToolbarPanelHeightMode.Fixed
      },
    hardwareKeyboardModeSwitchPending = softwareKeyboardVisible,
    hardwareKeyboardModeGenerationAtOpen = hardwareKeyboardModeGeneration,
  )

internal fun shouldHideSoftwareKeyboardWhenOpeningBottomPanel(
  softwareKeyboardVisible: Boolean
): Boolean = softwareKeyboardVisible

internal fun resolveEditorToolbarShouldRestoreSoftwareKeyboard(
  softwareKeyboardRestorePendingForPanel: Boolean,
  keyboardType: EditorKeyboardType = EditorKeyboardType.Software,
): Boolean = softwareKeyboardRestorePendingForPanel && keyboardType == EditorKeyboardType.Software

internal fun resolveEditorToolbarRestoreKeyboardType(
  keyboardType: EditorKeyboardType,
  softwareKeyboardRestorePendingForPanel: Boolean,
  hardwareKeyboardConnected: Boolean,
): EditorKeyboardType =
  if (softwareKeyboardRestorePendingForPanel && !hardwareKeyboardConnected) {
    EditorKeyboardType.Software
  } else {
    keyboardType
  }

internal fun resolveEditorToolbarRememberedKeyboardInsetClosePolicy(
  shouldRestoreSoftwareKeyboard: Boolean,
  softwareKeyboardSuppressedForPanel: Boolean,
  softwareKeyboardRestorePendingForPanel: Boolean = shouldRestoreSoftwareKeyboard,
): EditorToolbarRememberedKeyboardInsetClosePolicy =
  when {
    shouldRestoreSoftwareKeyboard ->
      EditorToolbarRememberedKeyboardInsetClosePolicy.KeepForExplicitSoftwareKeyboardRestore
    softwareKeyboardRestorePendingForPanel ->
      EditorToolbarRememberedKeyboardInsetClosePolicy.ClearBeforePanelClose
    softwareKeyboardSuppressedForPanel ->
      EditorToolbarRememberedKeyboardInsetClosePolicy.KeepForImplicitSoftwareKeyboardRestore
    else -> EditorToolbarRememberedKeyboardInsetClosePolicy.ClearBeforePanelClose
  }

internal fun shouldClearRememberedKeyboardInsetAfterHardwareKeyboardRestore(
  softwareKeyboardRestorePending: Boolean,
  bottomPanelVisible: Boolean,
  softwareKeyboardVisible: Boolean,
  keyboardType: EditorKeyboardType,
): Boolean =
  softwareKeyboardRestorePending &&
    !bottomPanelVisible &&
    !softwareKeyboardVisible &&
    keyboardType == EditorKeyboardType.Hardware

internal fun shouldCloseOpenEditorToolbarPanelWhenSoftwareKeyboardAppears(
  bottomPanelVisible: Boolean,
  previousSoftwareKeyboardVisible: Boolean,
  softwareKeyboardVisible: Boolean,
): Boolean = bottomPanelVisible && !previousSoftwareKeyboardVisible && softwareKeyboardVisible

internal fun shouldSwitchOpenEditorToolbarPanelToHardwareKeyboardMode(
  bottomPanelVisible: Boolean,
  softwareKeyboardVisible: Boolean,
  tracksImeInsetForPanel: Boolean,
  softwareKeyboardRestorePendingForPanel: Boolean,
  hardwareKeyboardModeGenerationAtOpen: Int,
  hardwareKeyboardModeGeneration: Int,
  hardwareKeyboardModeSwitchPendingForPanel: Boolean,
  keyboardType: EditorKeyboardType,
  hardwareKeyboardConnected: Boolean,
): Boolean =
  bottomPanelVisible &&
    !softwareKeyboardVisible &&
    tracksImeInsetForPanel &&
    hardwareKeyboardModeSwitchPendingForPanel &&
    hardwareKeyboardConnected &&
    (softwareKeyboardRestorePendingForPanel ||
      hardwareKeyboardModeGeneration > hardwareKeyboardModeGenerationAtOpen) &&
    keyboardType == EditorKeyboardType.Hardware
