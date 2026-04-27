package co.typie.screen.editor.editor.toolbar

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp

internal enum class EditorKeyboardType {
  Software,
  Hardware,
}

internal enum class EditorToolbarFixedAction {
  ClosePanel,
  HideToolbar,
  DismissInput,
}

@Composable internal expect fun rememberEditorKeyboardType(): EditorKeyboardType

internal fun isSoftwareKeyboardVisible(imeBottom: Dp, safeBottomInset: Dp): Boolean =
  imeBottom > safeBottomInset

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
