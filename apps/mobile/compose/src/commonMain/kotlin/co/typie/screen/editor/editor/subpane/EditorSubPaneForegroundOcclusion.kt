package co.typie.screen.editor.editor.subpane

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

internal data class EditorSubPaneForegroundOcclusion(
  val height: Dp,
  val headerRevealHeight: Dp = 0.dp,
)

internal fun computeEditorSubPaneForegroundOcclusion(
  trustedImeBottomInset: Dp,
  toolbarControlsOcclusion: Dp,
  bottomPanelOrKeyboardOcclusion: Dp,
  lastBottomPanelOcclusion: Dp,
  bottomPanelOpen: Boolean,
  panelTransitionRunning: Boolean,
  keyboardRestoreInset: Dp?,
  softwareKeyboardVisible: Boolean,
): EditorSubPaneForegroundOcclusion {
  val height =
    when {
      bottomPanelOpen -> toolbarControlsOcclusion + bottomPanelOrKeyboardOcclusion
      panelTransitionRunning ->
        toolbarControlsOcclusion + maxOf(bottomPanelOrKeyboardOcclusion, lastBottomPanelOcclusion)
      keyboardRestoreInset != null -> toolbarControlsOcclusion + bottomPanelOrKeyboardOcclusion
      softwareKeyboardVisible -> toolbarControlsOcclusion + trustedImeBottomInset
      else -> trustedImeBottomInset
    }
  val headerRevealHeight =
    if (
      bottomPanelOpen ||
        panelTransitionRunning ||
        keyboardRestoreInset != null ||
        softwareKeyboardVisible
    ) {
      EditorSubPaneHeaderRevealHeight
    } else {
      0.dp
    }

  return EditorSubPaneForegroundOcclusion(height = height, headerRevealHeight = headerRevealHeight)
}
