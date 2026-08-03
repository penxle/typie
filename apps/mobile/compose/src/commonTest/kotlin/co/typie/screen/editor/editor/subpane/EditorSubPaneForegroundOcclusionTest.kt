package co.typie.screen.editor.editor.subpane

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorSubPaneForegroundOcclusionTest {
  @Test
  fun bottomPanelRevealsHeaderAboveItsFullOcclusion() {
    assertEquals(
      EditorSubPaneForegroundOcclusion(
        height = 330.dp,
        headerRevealHeight = EditorSubPaneHeaderRevealHeight,
      ),
      computeEditorSubPaneForegroundOcclusion(
        trustedImeBottomInset = 0.dp,
        toolbarControlsOcclusion = 50.dp,
        bottomPanelOrKeyboardOcclusion = 280.dp,
        lastBottomPanelOcclusion = 280.dp,
        bottomPanelOpen = true,
        panelTransitionRunning = true,
        keyboardRestoreInset = null,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun panelTransitionRetainsLastPanelOcclusionAndHeaderReveal() {
    assertEquals(
      EditorSubPaneForegroundOcclusion(
        height = 210.dp,
        headerRevealHeight = EditorSubPaneHeaderRevealHeight,
      ),
      computeEditorSubPaneForegroundOcclusion(
        trustedImeBottomInset = 0.dp,
        toolbarControlsOcclusion = 50.dp,
        bottomPanelOrKeyboardOcclusion = 0.dp,
        lastBottomPanelOcclusion = 160.dp,
        bottomPanelOpen = false,
        panelTransitionRunning = true,
        keyboardRestoreInset = null,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun keyboardRestoreRetainsOcclusionAndHeaderReveal() {
    assertEquals(
      EditorSubPaneForegroundOcclusion(
        height = 330.dp,
        headerRevealHeight = EditorSubPaneHeaderRevealHeight,
      ),
      computeEditorSubPaneForegroundOcclusion(
        trustedImeBottomInset = 0.dp,
        toolbarControlsOcclusion = 50.dp,
        bottomPanelOrKeyboardOcclusion = 280.dp,
        lastBottomPanelOcclusion = 280.dp,
        bottomPanelOpen = false,
        panelTransitionRunning = false,
        keyboardRestoreInset = 280.dp,
        softwareKeyboardVisible = false,
      ),
    )
    assertEquals(
      EditorSubPaneForegroundOcclusion(
        height = 160.dp,
        headerRevealHeight = EditorSubPaneHeaderRevealHeight,
      ),
      computeEditorSubPaneForegroundOcclusion(
        trustedImeBottomInset = 0.dp,
        toolbarControlsOcclusion = 0.dp,
        bottomPanelOrKeyboardOcclusion = 160.dp,
        lastBottomPanelOcclusion = 160.dp,
        bottomPanelOpen = false,
        panelTransitionRunning = false,
        keyboardRestoreInset = 160.dp,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun softwareKeyboardRevealsHeaderAboveImeAndToolbar() {
    assertEquals(
      EditorSubPaneForegroundOcclusion(
        height = 330.dp,
        headerRevealHeight = EditorSubPaneHeaderRevealHeight,
      ),
      computeEditorSubPaneForegroundOcclusion(
        trustedImeBottomInset = 280.dp,
        toolbarControlsOcclusion = 50.dp,
        bottomPanelOrKeyboardOcclusion = 280.dp,
        lastBottomPanelOcclusion = 280.dp,
        bottomPanelOpen = false,
        panelTransitionRunning = false,
        keyboardRestoreInset = null,
        softwareKeyboardVisible = true,
      ),
    )
  }

  @Test
  fun hiddenForegroundUsesTrustedImeWithoutHeaderReveal() {
    assertEquals(
      EditorSubPaneForegroundOcclusion(height = 0.dp),
      computeEditorSubPaneForegroundOcclusion(
        trustedImeBottomInset = 0.dp,
        toolbarControlsOcclusion = 0.dp,
        bottomPanelOrKeyboardOcclusion = 0.dp,
        lastBottomPanelOcclusion = 0.dp,
        bottomPanelOpen = false,
        panelTransitionRunning = false,
        keyboardRestoreInset = null,
        softwareKeyboardVisible = false,
      ),
    )
  }

  @Test
  fun headerRevealIncludesBottomClearance() {
    assertEquals(68.dp, EditorSubPaneHeaderRevealHeight)
  }
}
