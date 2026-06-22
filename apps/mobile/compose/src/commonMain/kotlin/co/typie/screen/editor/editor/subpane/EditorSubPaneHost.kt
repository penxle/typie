package co.typie.screen.editor.editor.subpane

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import co.typie.navigation.PlatformBackHandler
import co.typie.screen.editor.editor.subpane.relatednotes.RelatedNotesSheet

@Composable
internal fun EditorSubPaneHost(
  state: EditorSubPaneState,
  entityId: String,
  maxTopInset: Dp,
  safeBottomInset: Dp,
  trustedImeBottomInset: Dp,
  modifier: Modifier = Modifier,
) {
  PlatformBackHandler(
    enabled = state.activeKey != null && state.activeKey != EditorSubPaneKey.RelatedNotes
  ) {
    state.dismiss()
  }

  when (state.activeKey) {
    EditorSubPaneKey.RelatedNotes ->
      RelatedNotesSheet(
        entityId = entityId,
        maxTopInset = maxTopInset,
        safeBottomInset = safeBottomInset,
        trustedImeBottomInset = trustedImeBottomInset,
        onDismiss = state::dismiss,
        onLayoutInfoChanged = state::updateLayoutInfo,
        onLayoutInfoCleared = state::clearLayoutInfo,
        modifier = modifier,
      )
    EditorSubPaneKey.Spellcheck,
    EditorSubPaneKey.AiFeedback,
    null -> Unit
  }
}
