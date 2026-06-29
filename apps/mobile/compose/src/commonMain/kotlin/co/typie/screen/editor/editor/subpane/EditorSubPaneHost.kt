package co.typie.screen.editor.editor.subpane

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import co.typie.graphql.type.UserRole
import co.typie.navigation.PlatformBackHandler
import co.typie.screen.editor.editor.subpane.comments.CommentsSheet
import co.typie.screen.editor.editor.subpane.comments.EditorCommentsSession
import co.typie.screen.editor.editor.subpane.relatednotes.RelatedNotesSheet

internal data class CommentsSubPaneEnvironment(
  val session: EditorCommentsSession,
  val myId: String?,
  val myRole: UserRole?,
  val isOwner: Boolean,
)

@Composable
internal fun EditorSubPaneHost(
  state: EditorSubPaneState,
  entityId: String,
  comments: CommentsSubPaneEnvironment,
  maxTopInset: Dp,
  safeBottomInset: Dp,
  trustedImeBottomInset: Dp,
  modifier: Modifier = Modifier,
) {
  LaunchedEffect(state.activeKey, comments.session.model) {
    if (state.activeKey == EditorSubPaneKey.Comments && comments.session.model == null) {
      state.dismiss()
    }
  }

  PlatformBackHandler(
    enabled =
      state.activeKey != null &&
        state.activeKey != EditorSubPaneKey.RelatedNotes &&
        state.activeKey != EditorSubPaneKey.Comments
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
    EditorSubPaneKey.Comments ->
      if (comments.session.model != null) {
        CommentsSheet(
          model = comments.session.model,
          myId = comments.myId,
          myRole = comments.myRole,
          isOwner = comments.isOwner,
          pendingRequest = comments.session.pendingRequest,
          onPendingRequestConsumed = comments.session.consumePendingRequest,
          threadLocationById = comments.session.threadLocationById,
          composeLocation = comments.session.composeLocation,
          createEnabled = comments.session.topBarCreateEnabled,
          onFreezeCurrentSelection = comments.session.freezeCurrentSelection,
          onInputFocusChanged = comments.session.onInputFocusChanged,
          maxTopInset = maxTopInset,
          safeBottomInset = safeBottomInset,
          trustedImeBottomInset = trustedImeBottomInset,
          onDismiss = state::dismiss,
          onLayoutInfoChanged = state::updateLayoutInfo,
          onLayoutInfoCleared = state::clearLayoutInfo,
          modifier = modifier,
        )
      }
    EditorSubPaneKey.AiFeedback,
    null -> Unit
  }
}
