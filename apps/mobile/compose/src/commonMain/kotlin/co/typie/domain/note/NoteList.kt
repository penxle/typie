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
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.result.Result
import co.typie.ui.component.Text
import co.typie.ui.component.reorder.ReorderDrop
import co.typie.ui.component.reorder.ReorderableColumn
import co.typie.ui.component.reorder.ReorderableColumnState
import co.typie.ui.component.reorder.rememberReorderableColumnState
import co.typie.ui.component.reorder.reorderableDragHandle
import co.typie.ui.component.reorder.reorderableItem
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.Toast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal const val NoteEnterDurationMillis = 220
internal const val NoteExitDelayMillis = 250
internal const val NoteExitDurationMillis = 180

internal data class NoteListItem(
  val note: NoteCard_note,
  val expanded: Boolean,
  val isSaving: Boolean,
  val hasPendingColor: Boolean,
  val isDirty: Boolean,
  val isEntering: Boolean,
  val isExiting: Boolean,
  val isExitVisible: Boolean,
)

internal class NoteListActions(
  val onExpand: (NoteCard_note) -> Unit,
  val onCollapse: () -> Unit,
  val onContentChange: (String, String) -> Unit,
  val onBlur: (String) -> Unit,
  val onToggleStatus: (NoteCard_note) -> Unit,
  val onColorChange: (NoteCard_note, String) -> Unit,
  val onAddEntity: (NoteCard_note) -> Unit,
  val onEntityClick: (NoteCard_note, NoteLinkedEntity_entity) -> Unit,
  val onDelete: (NoteCard_note) -> Unit,
  val onMoveNote:
    suspend (noteId: String, lowerOrder: String?, upperOrder: String?) -> Result<Unit, Nothing>,
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
  emptyMessage: String,
  queryState: QueryState<*>,
  items: List<NoteListItem>,
  onEnterAnimationFinished: (String) -> Unit,
  onExitAnimationFinished: (String) -> Unit,
  reorderState: ReorderableColumnState<String>,
  noteColorOptions: List<NoteColorOption>,
  interactive: Boolean,
  reorderEnabled: Boolean = true,
  contentEditable: Boolean = true,
  actions: NoteListActions,
) {
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val isLoading = queryState !is QueryState.Success<*>
  val showEmptyState = queryState is QueryState.Success<*> && items.isEmpty()
  val displayedItems = displayOrderedNoteItems(items, reorderState.keys)

  if (showEmptyState) {
    NoteEmptyState(message = emptyMessage)
  } else {
    Skeleton(enabled = isLoading) {
      ReorderableColumn(state = reorderState, verticalArrangement = Arrangement.spacedBy(12.dp)) {
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

            LaunchedEffect(item.note.id, noteIsEntering) {
              if (noteIsEntering) {
                delay(NoteEnterDurationMillis.toLong())
                onEnterAnimationFinished(item.note.id)
              }
            }

            LaunchedEffect(item.note.id, noteIsExitVisible) {
              if (noteIsExitVisible) {
                delay((NoteExitDelayMillis + NoteExitDurationMillis).toLong())
                onExitAnimationFinished(item.note.id)
              }
            }

            val visibilityState =
              remember(item.note.id) { MutableTransitionState(initialState = !noteIsEntering) }
            visibilityState.targetState = !noteIsExiting
            val rowModifier =
              if (reorderState.isDragging && !reorderState.isDragging(item.note.id)) {
                Modifier.animateBounds(
                  lookaheadScope = this@ReorderableColumn,
                  boundsTransform = boundsTransform,
                )
              } else {
                Modifier
              }

            AnimatedVisibility(
              modifier =
                if (interactive) {
                  rowModifier.reorderableItem(state = reorderState, key = item.note.id)
                } else {
                  rowModifier
                },
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
              NoteCard(
                note = note,
                expanded = !isLoading && item.expanded,
                isDragging = !isLoading && interactive && reorderState.isDragging(note.id),
                content = noteContent,
                isSaving = !isLoading && item.isSaving,
                colorOption = colorOption,
                contentEditable = contentEditable,
                dragHandleModifier =
                  if (interactive && !isLoading) {
                    Modifier.reorderableDragHandle(
                      state = reorderState,
                      key = note.id,
                      enabled =
                        reorderEnabled &&
                          !noteIsExiting &&
                          !item.isSaving &&
                          !item.hasPendingColor &&
                          (!item.expanded || !item.isDirty),
                      onDragStopped = { commit ->
                        scope.launch {
                          handleReorderCommit(
                            noteId = note.id,
                            commit = commit,
                            displayedNotes = displayedItems.map(NoteListItem::note),
                            reorderState = reorderState,
                            toast = toast,
                            moveNote = { lowerOrder, upperOrder ->
                              actions.onMoveNote(note.id, lowerOrder, upperOrder)
                            },
                          )
                        }
                      },
                    )
                  } else {
                    Modifier
                  },
                onExpand = { if (interactive && !isLoading) actions.onExpand(note) },
                onCollapse = { if (interactive && !isLoading) actions.onCollapse() },
                onContentChange = { nextValue ->
                  if (interactive && !isLoading) {
                    actions.onContentChange(note.id, nextValue)
                  }
                },
                onBlur = { if (interactive && !isLoading) actions.onBlur(note.id) },
                onToggleStatus = { if (interactive && !isLoading) actions.onToggleStatus(note) },
                onColorChange = { nextColor ->
                  if (interactive && !isLoading) {
                    actions.onColorChange(note, nextColor)
                  }
                },
                onAddEntity = { if (interactive && !isLoading) actions.onAddEntity(note) },
                onEntityClick = { entity ->
                  if (interactive && !isLoading) {
                    actions.onEntityClick(note, entity)
                  }
                },
                onDelete = { if (interactive && !isLoading) actions.onDelete(note) },
                noteColorOptions = noteColorOptions,
              )
            }
          }
        }
      }
    }
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

private suspend fun handleReorderCommit(
  noteId: String,
  commit: ReorderDrop<String>?,
  displayedNotes: List<NoteCard_note>,
  reorderState: ReorderableColumnState<String>,
  toast: Toast,
  moveNote: suspend (lowerOrder: String?, upperOrder: String?) -> Result<Unit, Nothing>,
) {
  if (commit == null) {
    return
  }

  val reorderedNotes =
    commit.orderedKeys.mapNotNull { orderedNoteId ->
      displayedNotes.firstOrNull { it.id == orderedNoteId }
    }
  val moveOrders = resolveMovedNoteOrders(reorderedNotes, movedNoteId = noteId) ?: return

  when (moveNote(moveOrders.lowerOrder, moveOrders.upperOrder)) {
    is Result.Ok -> Unit
    is Result.Err,
    is Result.Exception -> {
      toast.show(ToastType.Error, "순서를 바꿀 수 없어요.")
    }
  }
}

internal data class NoteMoveOrders(val lowerOrder: String?, val upperOrder: String?)

internal fun resolveMovedNoteOrders(
  orderedNotes: List<NoteCard_note>,
  movedNoteId: String,
): NoteMoveOrders? {
  val movedIndex = orderedNotes.indexOfFirst { it.id == movedNoteId }
  if (movedIndex == -1) {
    return null
  }

  return NoteMoveOrders(
    lowerOrder = orderedNotes.getOrNull(movedIndex - 1)?.order,
    upperOrder = orderedNotes.getOrNull(movedIndex + 1)?.order,
  )
}

internal fun displayOrderedNotes(
  notes: List<NoteCard_note>,
  orderedKeys: List<String>,
): List<NoteCard_note> {
  val notesById = notes.associateBy { it.id }
  if (orderedKeys.size != notesById.size) {
    return notes
  }

  val orderedNotes = orderedKeys.mapNotNull(notesById::get)
  return if (orderedNotes.size == notesById.size) orderedNotes else notes
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
