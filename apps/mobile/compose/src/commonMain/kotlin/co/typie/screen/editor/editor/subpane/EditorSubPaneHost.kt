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
  siteId: String,
  editorMutationEnabled: Boolean,
  comments: CommentsSubPaneEnvironment,
  maxTopInset: Dp,
  safeBottomInset: Dp,
  trustedImeBottomInset: Dp,
  editorFocused: Boolean,
  foregroundOcclusion: EditorSubPaneForegroundOcclusion,
  modifier: Modifier = Modifier,
) {
  val editor = LocalEditorRuntime.current.editor
  val active = state.active
  val selection = editor?.appliedState?.selection

  LaunchedEffect(active, selection) { state.dismissTableAxisActionsIfSelectionChanged(selection) }
  LaunchedEffect(active, editorMutationEnabled) {
    if (!editorMutationEnabled) {
      state.dismissTableAxisActions()
    }
  }

  LaunchedEffect(active, comments.session.model) {
    if (active == EditorSubPane.Comments && comments.session.model == null) {
      state.dismiss()
    }
  }

  when (active) {
    EditorSubPane.RelatedNotes ->
      RelatedNotesSheet(
        entityId = entityId,
        siteId = siteId,
        maxTopInset = maxTopInset,
        safeBottomInset = safeBottomInset,
        trustedImeBottomInset = trustedImeBottomInset,
        editorFocused = editorFocused,
        foregroundOcclusion = foregroundOcclusion,
        onDismissStarted = state::beginDismiss,
        onDismiss = state::dismiss,
        onLayoutInfoChanged = state::updateLayoutInfo,
        onLayoutInfoCleared = state::clearLayoutInfo,
        registerRouteRemovalPreparation = state::registerRouteRemovalPreparation,
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
          ensureMutationSubscription = comments.session.ensureMutationSubscription,
          onInputFocusChanged = comments.session.onInputFocusChanged,
          maxTopInset = maxTopInset,
          safeBottomInset = safeBottomInset,
          trustedImeBottomInset = trustedImeBottomInset,
          editorFocused = editorFocused,
          foregroundOcclusion = foregroundOcclusion,
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
        currentBackgroundColor = editor?.publishedState?.modifierState?.cellBackgroundColor,
        dismissRequestVersion = state.dismissRequestVersion,
        onAction = tableAction@{ message ->
            if (!editorMutationEnabled) return@tableAction
            val currentEditor = editor ?: return@tableAction
            try {
              currentEditor.updateNow { enqueue(message) }
            } catch (error: Throwable) {
              if (!currentEditor.terminal) throw error
              return@tableAction
            }
            currentEditor.focus()
          },
        onDismissStarted = state::beginDismiss,
        onDismissCancelled = state::cancelDismiss,
        onDismiss = state::dismiss,
        onLayoutInfoChanged = state::updateLayoutInfo,
        onLayoutInfoCleared = state::clearLayoutInfo,
        modifier = modifier,
      )
    null -> Unit
  }
}
