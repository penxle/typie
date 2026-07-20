package co.typie.screen.editor.editor.subpane

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.graphql.type.UserRole
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
  onAuxiliaryInputFocused: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val editor = LocalEditorRuntime.current.editor
  val active = state.active

  LaunchedEffect(active, comments.session.model) {
    if (active == EditorSubPane.Comments && comments.session.model == null) {
      state.dismiss()
    }
  }

  when (active) {
    EditorSubPane.RelatedNotes ->
      RelatedNotesSheet(
        entityId = entityId,
        maxTopInset = maxTopInset,
        safeBottomInset = safeBottomInset,
        trustedImeBottomInset = trustedImeBottomInset,
        onInputFocused = onAuxiliaryInputFocused,
        onDismissStarted = state::beginDismiss,
        onDismiss = state::dismiss,
        onLayoutInfoChanged = state::updateLayoutInfo,
        onLayoutInfoCleared = state::clearLayoutInfo,
        modifier = modifier,
      )
    EditorSubPane.Comments ->
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
          onInputFocusChanged = { focused ->
            comments.session.onInputFocusChanged(focused)
            if (focused) onAuxiliaryInputFocused()
          },
          maxTopInset = maxTopInset,
          safeBottomInset = safeBottomInset,
          trustedImeBottomInset = trustedImeBottomInset,
          onDismissStarted = state::beginDismiss,
          onDismiss = state::dismiss,
          onLayoutInfoChanged = state::updateLayoutInfo,
          onLayoutInfoCleared = state::clearLayoutInfo,
          modifier = modifier,
        )
      }
    is EditorSubPane.TableAxisActions ->
      EditorTableAxisActionsPane(
        pane = active,
        currentBackgroundColor = editor?.state?.modifierState?.cellBackgroundColor,
        dismissRequestVersion = state.dismissRequestVersion,
        onAction = { message ->
          editor?.sync { enqueue(message) }
          editor?.focus()
        },
        onDismissStarted = state::beginDismiss,
        onDismiss = state::dismiss,
        onLayoutInfoChanged = state::updateLayoutInfo,
        onLayoutInfoCleared = state::clearLayoutInfo,
        modifier = modifier,
      )
    null -> Unit
  }
}
