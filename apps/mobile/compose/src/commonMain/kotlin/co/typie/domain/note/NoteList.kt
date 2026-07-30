package co.typie.domain.note

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateBounds
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.unit.dp
import co.touchlab.kermit.Logger
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.network.isRecoverableNetworkError
import co.typie.result.Result
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Text
import co.typie.ui.component.reorder.ReorderableColumn
import co.typie.ui.component.reorder.ReorderableColumnState
import co.typie.ui.component.reorder.rememberReorderableColumnState
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.flow.first

internal const val NoteEnterDurationMillis = 220
internal const val NoteExitDelayMillis = 250
internal const val NoteExitDurationMillis = 180

internal data class NoteListItem(
  val note: NoteCard_note,
  val expanded: Boolean,
  val isSaving: Boolean,
  val saveStatus: NoteSaveStatus,
  val hasPendingColor: Boolean,
  val isDirty: Boolean,
  val isDeleting: Boolean,
  val isChangingStatus: Boolean,
  val autoFocusContent: Boolean,
  val isEntering: Boolean,
  val isExiting: Boolean,
  val isExitVisible: Boolean,
)

internal class NoteListActions(
  val onExpand: (NoteCard_note) -> Unit,
  val onCollapse: () -> Unit,
  val onCreateNote: () -> Unit,
  val onContentChange: (NoteCard_note, String) -> Unit,
  val onBlur: (NoteCard_note) -> Unit,
  val onToggleStatus: (NoteCard_note) -> Unit,
  val onColorChange: (NoteCard_note, String) -> Unit,
  val onAddEntity: (NoteCard_note) -> Unit,
  val onEntityClick: (NoteCard_note, NoteLinkedEntity_entity) -> Unit,
  val onDelete: (NoteCard_note) -> Unit,
  val onMoveNote:
    suspend (note: NoteCard_note, lowerOrder: String?, upperOrder: String?) -> Result<
        String,
        Nothing,
      >,
)

@Composable
internal fun rememberNoteListReorderState(
  items: List<NoteListItem>,
  scrollState: ScrollState,
): ReorderableColumnState<String> =
  rememberReorderableColumnState(
    keys = items.map { it.note.id },
    verticalScrollableState = scrollState,
  )

@Composable
internal fun NoteList(
  identity: NoteListIdentity,
  emptyMessage: String,
  queryState: QueryState<*>,
  items: List<NoteListItem>,
  authoritativeNotes: List<NoteCard_note>,
  onEnterAnimationFinished: (String) -> Unit,
  onExitAnimationFinished: (String) -> Unit,
  reorderState: ReorderableColumnState<String>,
  noteColorOptions: List<NoteColorOption>,
  interactive: Boolean,
  onRetry: suspend () -> Unit,
  reorderEnabled: Boolean = true,
  contentEditable: Boolean = true,
  actions: NoteListActions,
) {
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val noteReorderState =
    remember(scope, reorderState) { NoteReorderState(scope = scope, reorderState = reorderState) }
  val isLoading = queryState is QueryState.Loading
  val showErrorState = queryState is QueryState.Error
  val showEmptyState = queryState is QueryState.Success<*> && items.isEmpty()
  val displayedItems = displayOrderedNoteItems(items, reorderState.keys)

  SideEffect { noteReorderState.sync(identity = identity, notes = authoritativeNotes) }
  DisposableEffect(noteReorderState) { onDispose(noteReorderState::dispose) }
  LaunchedEffect(noteReorderState, toast) {
    noteReorderState.failures.collect { error ->
      Logger.e(error) { "Failed to reorder note" }
      if (!error.isRecoverableNetworkError()) {
        Sentry.captureException(error)
      }
      toast.show(ToastType.Error, "순서를 바꾸지 못했어요.")
    }
  }

  if (showErrorState) {
    NoteQueryError(onRetry = onRetry)
  } else if (showEmptyState) {
    NoteEmptyState(message = emptyMessage)
  } else {
    Skeleton(enabled = isLoading) {
      ReorderableColumn(state = reorderState) {
        val boundsTransform = remember {
          androidx.compose.animation.BoundsTransform { _, _ ->
            spring(dampingRatio = 0.9f, stiffness = Spring.StiffnessMedium)
          }
        }

        displayedItems.forEach { item ->
          key(item.note.id) {
            val note = item.note
            val colorOption = noteColorOptions.resolve(note.color)
            val noteContent = note.content
            val noteIsEntering = !isLoading && item.isEntering
            val noteIsExiting = !isLoading && item.isExiting
            val noteIsExitVisible = !isLoading && item.isExitVisible
            val cardExpanded = !isLoading && item.expanded
            val noteInteractionEnabled =
              interactive &&
                !isLoading &&
                !item.isDeleting &&
                !item.isChangingStatus &&
                !noteIsExiting

            val visibilityState =
              remember(item.note.id) { MutableTransitionState(initialState = !noteIsEntering) }
            visibilityState.targetState = !noteIsExiting

            LaunchedEffect(item.note.id, noteIsEntering) {
              if (!noteIsEntering) return@LaunchedEffect
              snapshotFlow {
                  visibilityState.isIdle &&
                    visibilityState.currentState &&
                    visibilityState.targetState
                }
                .first { it }
              onEnterAnimationFinished(item.note.id)
            }

            LaunchedEffect(item.note.id, noteIsExitVisible) {
              if (!noteIsExitVisible) return@LaunchedEffect
              snapshotFlow {
                  visibilityState.isIdle &&
                    !visibilityState.currentState &&
                    !visibilityState.targetState
                }
                .first { it }
              onExitAnimationFinished(item.note.id)
            }

            val rowModifier =
              if (
                !visibilityState.isIdle ||
                  noteIsEntering ||
                  noteIsExiting ||
                  reorderState.isDragging(item.note.id) ||
                  reorderState.isSettling(item.note.id)
              ) {
                Modifier
              } else {
                Modifier.animateBounds(
                  lookaheadScope = this@ReorderableColumn,
                  boundsTransform = boundsTransform,
                )
              }
            val itemModifier =
              if (interactive) {
                rowModifier.reorderableItem(state = reorderState, key = item.note.id)
              } else {
                rowModifier
              }
            val presenceModifier =
              if (interactive && !noteIsExiting) {
                itemModifier
              } else {
                itemModifier.focusProperties { canFocus = false }.clearAndSetSemantics {}
              }

            AnimatedVisibility(
              modifier = presenceModifier,
              visibleState = visibilityState,
              enter =
                fadeIn(animationSpec = tween(durationMillis = NoteEnterDurationMillis)) +
                  expandVertically(
                    animationSpec = tween(durationMillis = NoteEnterDurationMillis),
                    expandFrom = Alignment.Top,
                  ),
              exit =
                fadeOut(
                  animationSpec =
                    tween(
                      durationMillis = NoteExitDurationMillis,
                      delayMillis = NoteExitDelayMillis,
                    )
                ) +
                  slideOutVertically(
                    animationSpec =
                      tween(
                        durationMillis = NoteExitDurationMillis,
                        delayMillis = NoteExitDelayMillis,
                      ),
                    targetOffsetY = { -it / 6 },
                  ) +
                  shrinkVertically(
                    animationSpec =
                      tween(
                        durationMillis = NoteExitDurationMillis,
                        delayMillis = NoteExitDelayMillis,
                      ),
                    shrinkTowards = Alignment.Top,
                  ),
            ) {
              Box(modifier = Modifier.padding(bottom = 12.dp)) {
                NoteCard(
                  note = note,
                  expanded = cardExpanded,
                  autoFocusContent = item.autoFocusContent,
                  isDragging = !isLoading && interactive && reorderState.isDragging(note.id),
                  content = noteContent,
                  saveStatus = if (isLoading) NoteSaveStatus.NONE else item.saveStatus,
                  colorOption = colorOption,
                  contentEditable =
                    contentEditable && !item.isDeleting && !item.isChangingStatus && !noteIsExiting,
                  dragHandleModifier =
                    if (interactive && !isLoading) {
                      Modifier.reorderableDragHandle(
                        state = reorderState,
                        key = note.id,
                        enabled =
                          reorderEnabled &&
                            noteInteractionEnabled &&
                            !item.isSaving &&
                            !item.hasPendingColor &&
                            (!item.expanded || !item.isDirty),
                        onDragStopped = { commit ->
                          if (commit == null) return@reorderableDragHandle
                          noteReorderState.commit(commit, actions.onMoveNote)
                        },
                      )
                    } else {
                      Modifier
                    },
                  onExpand = { if (noteInteractionEnabled) actions.onExpand(note) },
                  onCollapse = { if (noteInteractionEnabled) actions.onCollapse() },
                  onCreateNote = {
                    if (interactive && !isLoading) {
                      actions.onCreateNote()
                    }
                  },
                  onContentChange = { nextValue ->
                    if (noteInteractionEnabled) {
                      actions.onContentChange(note, nextValue)
                    }
                  },
                  onBlur = { if (noteInteractionEnabled) actions.onBlur(note) },
                  onToggleStatus = {
                    if (noteInteractionEnabled) {
                      actions.onToggleStatus(note)
                    }
                  },
                  onColorChange = { nextColor ->
                    if (noteInteractionEnabled) {
                      actions.onColorChange(note, nextColor)
                    }
                  },
                  onAddEntity = { if (noteInteractionEnabled) actions.onAddEntity(note) },
                  onEntityClick = { entity ->
                    if (noteInteractionEnabled) {
                      actions.onEntityClick(note, entity)
                    }
                  },
                  onDelete = { if (noteInteractionEnabled) actions.onDelete(note) },
                  isDeleting = item.isDeleting,
                  noteColorOptions = noteColorOptions,
                )
              }
            }
          }
        }
      }
    }
  }
}

@Composable
private fun NoteQueryError(onRetry: suspend () -> Unit) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    NoteEmptyState(message = "노트를 불러오지 못했어요.")
    Button(text = "다시 시도", variant = ButtonVariant.Secondary, onClick = onRetry)
  }
}

@Composable
private fun NoteEmptyState(message: String) {
  Box(
    modifier =
      Modifier.fillMaxWidth()
        .height(110.dp)
        .clip(AppShapes.rounded(AppShapes.md))
        .background(AppTheme.colors.surfaceDefault),
    contentAlignment = Alignment.Center,
  ) {
    Text(message, style = AppTheme.typography.action, color = AppTheme.colors.textMuted)
  }
}

internal fun displayOrderedNoteItems(
  items: List<NoteListItem>,
  orderedKeys: List<String>,
): List<NoteListItem> {
  val itemsById = items.associateBy { it.note.id }
  if (orderedKeys.size != itemsById.size) {
    return items
  }

  val orderedItems = orderedKeys.mapNotNull(itemsById::get)
  return if (orderedItems.size == itemsById.size) orderedItems else items
}
