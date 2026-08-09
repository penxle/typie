package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ToolbarPlacementTest {
  @Test
  fun keyboardOwnedPlacementUsesTheLatestVisualInset() {
    assertEquals(
      280,
      resolveEditorToolbarVisualBottomInset(
        inputSpaceOwnsPlacement = true,
        visualImeBottom = 280,
        safeBottomInset = 46,
        retainedBottomInset = 240,
      ),
    )
  }

  @Test
  fun panelOwnedPlacementIgnoresTheKeyboardInset() {
    assertEquals(
      46,
      resolveEditorToolbarVisualBottomInset(
        inputSpaceOwnsPlacement = false,
        visualImeBottom = 280,
        safeBottomInset = 46,
        retainedBottomInset = 46,
      ),
    )
  }

  @Test
  fun keyboardRestoreKeepsPanelSpaceAsThePlacementOwner() {
    assertFalse(editorToolbarInputSpaceOwnsPlacement(bottomPanelLayoutHeight = 188.dp))
  }

  @Test
  fun liveInputOwnsPlacementAfterPanelSpaceIsReleased() {
    assertTrue(editorToolbarInputSpaceOwnsPlacement(bottomPanelLayoutHeight = 0.dp))
  }

  @Test
  fun exitingPanelRetainsPlacementOwnershipWhenTheKeyboardIsAlreadyVisible() {
    val bottomPanelLayoutHeight =
      resolveEditorToolbarBottomPanelLayoutHeight(
        activeBottomPanelContainerHeight = null,
        restoringKeyboard = false,
        panelTransitionIdle = false,
        lastBottomPanelContainerHeight = 188.dp,
      )

    assertFalse(editorToolbarInputSpaceOwnsPlacement(bottomPanelLayoutHeight))
    assertEquals(
      46,
      resolveEditorToolbarVisualBottomInset(
        inputSpaceOwnsPlacement = editorToolbarInputSpaceOwnsPlacement(bottomPanelLayoutHeight),
        visualImeBottom = 280,
        safeBottomInset = 46,
        retainedBottomInset = 46,
      ),
    )
  }

  @Test
  fun exitingPanelKeepsItsLayoutHeightUntilVisibilityTransitionCompletes() {
    assertEquals(
      188.dp,
      resolveEditorToolbarBottomPanelLayoutHeight(
        activeBottomPanelContainerHeight = null,
        restoringKeyboard = false,
        panelTransitionIdle = false,
        lastBottomPanelContainerHeight = 188.dp,
      ),
    )
  }

  @Test
  fun settledPanelExitReleasesItsLayoutHeightWithoutAnimation() {
    assertEquals(
      0.dp,
      resolveEditorToolbarBottomPanelLayoutHeight(
        activeBottomPanelContainerHeight = null,
        restoringKeyboard = false,
        panelTransitionIdle = true,
        lastBottomPanelContainerHeight = 188.dp,
      ),
    )
  }
}
