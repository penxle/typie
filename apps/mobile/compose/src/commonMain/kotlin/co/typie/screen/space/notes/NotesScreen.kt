package co.typie.screen.space.notes

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.isFolder
import co.typie.domain.note.NoteEntityPickerSheet
import co.typie.domain.note.NoteEntityPickerStops
import co.typie.domain.note.NoteLinkedEntityActionsSheet
import co.typie.domain.note.NoteList
import co.typie.domain.note.NoteListActions
import co.typie.domain.note.NoteListItem
import co.typie.domain.note.emptyMessage
import co.typie.domain.note.filterLabel
import co.typie.domain.note.rememberNoteColorOptions
import co.typie.domain.note.rememberNoteListReorderState
import co.typie.domain.note.toggled
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.graphql.type.NoteStatus
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.navigation.Nav
import co.typie.result.Result
import co.typie.route.Route
import co.typie.shell.MainBottomBarPillEntry
import co.typie.shell.MainBottomBarPillKey
import co.typie.shell.MainDrawerTrigger
import co.typie.shell.MainDrawerTriggerLeadingKey
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.BottomBarAction
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.reorder.reorderableViewport
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.TopBarDefaults
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private object NotesFilterTopBarTrailingKey

@Composable
fun NotesScreen() {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val model = viewModel { NotesViewModel() }
  val noteEditState = model.noteEditState
  val scrollState = rememberScrollState()
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val siteId = model.siteId
  val noteColorOptions = rememberNoteColorOptions()

  DisposableEffect(noteEditState, model) {
    onDispose {
      noteEditState.dispose(
        savePendingContent = model::savePendingNoteContent,
        savePendingColor = model::savePendingNoteColor,
      )
    }
  }

  suspend fun saveNoteContent(noteId: String, content: String): Boolean {
    return when (val result = model.updateNoteContent(noteId = noteId, content = content)) {
      is Result.Ok -> {
        noteEditState.commitServerSnapshot(result.value)
        true
      }

      is Result.Err,
      is Result.Exception -> {
        toast.show(ToastType.Error, "노트를 저장할 수 없어요.")
        false
      }
    }
  }

  suspend fun saveNoteColor(noteId: String, color: String): Boolean {
    return when (val result = model.updateNoteColor(noteId = noteId, color = color)) {
      is Result.Ok -> {
        noteEditState.commitServerSnapshot(result.value)
        true
      }

      is Result.Err,
      is Result.Exception -> {
        toast.show(ToastType.Error, "색상을 바꿀 수 없어요.")
        false
      }
    }
  }

  suspend fun flushNoteEdits(noteId: String): Boolean {
    return noteEditState.flush(
      noteId = noteId,
      saveContent = ::saveNoteContent,
      saveColor = ::saveNoteColor,
    )
  }

  suspend fun collapseExpandedNote(): Boolean {
    return noteEditState.collapse(saveContent = ::saveNoteContent, saveColor = ::saveNoteColor)
  }

  suspend fun handleExpandNote(note: NoteCard_note) {
    val expandedNoteId = noteEditState.expandedNoteId
    if (expandedNoteId != null && expandedNoteId != note.id && !flushNoteEdits(expandedNoteId)) {
      return
    }

    noteEditState.open(note = note)
  }

  suspend fun handleFilterSelection(nextStatus: NoteStatus) {
    if (nextStatus == model.filterStatus || nextStatus == NoteStatus.UNKNOWN__) {
      return
    }

    if (!collapseExpandedNote()) {
      return
    }

    model.updateFilterStatus(nextStatus)
    scrollState.scrollTo(0)
  }

  suspend fun handleCreateNote() {
    if (siteId == null) {
      return
    }

    if (!collapseExpandedNote()) {
      return
    }

    if (model.filterStatus == NoteStatus.RESOLVED) {
      model.updateFilterStatus(NoteStatus.OPEN)
      scrollState.scrollTo(0)
    }

    when (val result = model.createNote()) {
      is Result.Ok -> {
        model.listState(NoteStatus.OPEN).markEntering(result.value)
        noteEditState.open(note = result.value)
        model.refetch()
      }

      is Result.Err,
      is Result.Exception -> {
        toast.show(ToastType.Error, "노트를 만들 수 없어요.")
      }
    }
  }

  suspend fun handleDeleteNote(note: NoteCard_note, sceneStatus: NoteStatus) {
    val confirmed =
      dialog.confirm(
        title = "노트 삭제",
        message = "이 노트를 삭제하시겠어요?\n복구할 수 없어요.",
        confirmText = "삭제",
        confirmIsDestructive = true,
      )

    if (confirmed !is DialogResult.Resolved) {
      return
    }

    noteEditState.cancelPendingSaves(note.id)
    model.listState(sceneStatus).markExiting(note)

    when (model.deleteNote(note.id)) {
      is Result.Ok -> {
        noteEditState.remove(note.id)
        model.refetch()
        toast.show(ToastType.Success, "노트를 삭제했어요.")
      }

      is Result.Err,
      is Result.Exception -> {
        model.listState(sceneStatus).remove(note.id)
        toast.show(ToastType.Error, "노트를 삭제할 수 없어요.")
      }
    }
  }

  suspend fun handleToggleStatus(note: NoteCard_note, sceneStatus: NoteStatus) {
    if (!flushNoteEdits(note.id)) {
      return
    }

    val nextStatus = note.status.toggled()
    model.listState(sceneStatus).markExiting(note.copy(status = nextStatus))

    when (val result = model.updateNoteStatus(noteId = note.id, status = nextStatus)) {
      is Result.Ok -> {
        noteEditState.commitServerSnapshot(result.value)
        noteEditState.clearExpanded(note.id)
        model.listState(nextStatus).expectEntry(result.value)
        model.refetch()
      }

      is Result.Err,
      is Result.Exception -> {
        model.listState(sceneStatus).remove(note.id)
        toast.show(ToastType.Error, "상태를 바꿀 수 없어요.")
      }
    }
  }

  fun handleColorChange(note: NoteCard_note, color: String) {
    if (note.color == color) {
      return
    }

    noteEditState.updateColor(noteId = note.id, value = color, save = ::saveNoteColor)
  }

  suspend fun handleAddEntity(noteId: String, entityId: String): Boolean {
    if (!flushNoteEdits(noteId)) {
      return false
    }

    return when (val result = model.addNoteEntity(noteId = noteId, entityId = entityId)) {
      is Result.Ok -> {
        noteEditState.commitServerSnapshot(result.value)
        true
      }

      is Result.Err,
      is Result.Exception -> {
        toast.show(ToastType.Error, "연결을 추가할 수 없어요.")
        false
      }
    }
  }

  suspend fun handleRemoveEntity(noteId: String, entityId: String): Boolean {
    if (!flushNoteEdits(noteId)) {
      return false
    }

    return when (val result = model.removeNoteEntity(noteId = noteId, entityId = entityId)) {
      is Result.Ok -> {
        noteEditState.commitServerSnapshot(result.value)
        true
      }

      is Result.Err,
      is Result.Exception -> {
        toast.show(ToastType.Error, "연결을 해제할 수 없어요.")
        false
      }
    }
  }

  fun presentEntityPicker(note: NoteCard_note) {
    if (siteId == null) {
      return
    }

    scope.launch {
      sheet.present(stops = NoteEntityPickerStops) {
        NoteEntityPickerSheet(
          linkedEntityIds = note.entities.mapTo(mutableSetOf()) { it.noteLinkedEntity_entity.id },
          onAddEntity = { entityId -> handleAddEntity(note.id, entityId) },
          onRemoveEntity = { entityId -> handleRemoveEntity(note.id, entityId) },
        )
      }
    }
  }

  fun presentLinkedEntityActions(note: NoteCard_note, linkedEntity: NoteLinkedEntity_entity) {
    scope.launch {
      sheet.present {
        NoteLinkedEntityActionsSheet(
          linkedEntity = linkedEntity,
          onOpen = {
            scope.launch {
              if (linkedEntity.entityRow_entity.isFolder())
                nav.navigate(Route.Folder(linkedEntity.id))
              else nav.navigate(Route.Editor(linkedEntity.id))
            }
          },
          onUnlink = { scope.launch { handleRemoveEntity(note.id, linkedEntity.id) } },
        )
      }
    }
  }

  ProvideTopBar(
    leadingKey = MainDrawerTriggerLeadingKey,
    leading = { MainDrawerTrigger() },
    center = { Text("노트", style = AppTheme.typography.title) },
    trailingKey = NotesFilterTopBarTrailingKey,
    trailing = {
      NotesFilterPopover(
        selectedStatus = model.filterStatus,
        onSelect = { nextStatus -> scope.launch { handleFilterSelection(nextStatus) } },
      )
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  ProvideBottomBar(
    pillKey = MainBottomBarPillKey,
    pill = MainBottomBarPillEntry,
    action =
      BottomBarAction(
        icon = Typie.StickyNotePlus,
        onClick = { scope.launch { handleCreateNote() } },
      ),
  )

  Screen(loadable = model.query, background = AppTheme.colors.surfaceCanvas) { contentPadding ->
    Crossfade(
      targetState = model.filterStatus,
      modifier = Modifier,
      animationSpec = tween(durationMillis = 200),
    ) { status ->
      val listState = model.listState(status)
      val renderedNotes = listState.merge(model.notes(status)).map(noteEditState::overlay)
      val listItems = renderedNotes.map { note ->
        NoteListItem(
          note = note,
          expanded = noteEditState.expandedNoteId == note.id,
          isSaving = noteEditState.isSaving(note.id) || noteEditState.isSavingColor(note.id),
          hasPendingColor = noteEditState.hasPendingColor(note.id),
          isDirty = noteEditState.isDirty(note.id),
          isEntering = listState.isEntering(note.id),
          isExiting = listState.isExiting(note.id),
          isExitVisible = listState.isExitVisible(note.id),
        )
      }
      val listActions =
        NoteListActions(
          onExpand = { note -> scope.launch { handleExpandNote(note) } },
          onCollapse = { scope.launch { collapseExpandedNote() } },
          onContentChange = { noteId, content ->
            noteEditState.updateContent(noteId = noteId, value = content, save = ::saveNoteContent)
          },
          onBlur = { noteId -> scope.launch { flushNoteEdits(noteId) } },
          onToggleStatus = { note -> scope.launch { handleToggleStatus(note, status) } },
          onColorChange = ::handleColorChange,
          onAddEntity = ::presentEntityPicker,
          onEntityClick = ::presentLinkedEntityActions,
          onDelete = { note -> scope.launch { handleDeleteNote(note, status) } },
          onMoveNote = { noteId, lowerOrder, upperOrder ->
            model.moveNote(noteId = noteId, lowerOrder = lowerOrder, upperOrder = upperOrder)
          },
        )
      val reorderState = rememberNoteListReorderState(items = listItems, scrollState = scrollState)
      val reorderViewportTopInset =
        maxOf(
          0.dp,
          contentPadding.calculateTopPadding() -
            TopBarDefaults.BlurFadeHeight -
            TopBarDefaults.ContentTopSpacing,
        )
      val reorderViewportBottomInset =
        WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding() + 72.dp

      Box(
        modifier =
          Modifier.fillMaxSize()
            .reorderableViewport(
              state = reorderState,
              viewportTopInset = reorderViewportTopInset,
              viewportBottomInset = reorderViewportBottomInset,
            )
            .imePadding()
      ) {
        Column(
          modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
          verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          Skeleton.Keep { Text(text = "노트", style = AppTheme.typography.display) }

          NoteList(
            emptyMessage = status.emptyMessage(),
            queryState = model.queryState(status),
            items = listItems,
            onEnterAnimationFinished = listState::finishEntering,
            onExitAnimationFinished = listState::finishExiting,
            reorderState = reorderState,
            noteColorOptions = noteColorOptions,
            interactive = status == model.filterStatus,
            actions = listActions,
          )

          Spacer(Modifier.height(140.dp))
        }
      }
    }

    ToastAnchor(
      modifier =
        Modifier.align(Alignment.BottomCenter)
          .navigationBarsPadding()
          .padding(bottom = BottomBarDefaults.BarAreaHeight)
    )
  }
}

@Composable
private fun NotesFilterPopover(selectedStatus: NoteStatus, onSelect: (NoteStatus) -> Unit) {
  PopoverMenu(anchor = { TopBarButton(icon = Lucide.ListFilter) }) {
    listOf(NoteStatus.OPEN, NoteStatus.RESOLVED).forEach { status ->
      item(
        content = {
          Row(
            modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            Icon(
              icon = if (status == NoteStatus.RESOLVED) Lucide.CircleCheck else Lucide.Circle,
              modifier = Modifier.size(18.dp),
              tint = AppTheme.colors.textMuted,
            )
            Text(
              text = status.filterLabel(),
              modifier = Modifier.weight(1f),
              style = AppTheme.typography.action,
            )
            Box(modifier = Modifier.width(28.dp), contentAlignment = Alignment.CenterEnd) {
              if (selectedStatus == status) {
                Icon(
                  icon = Lucide.Check,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textDefault,
                )
              }
            }
          }
        }
      ) {
        onSelect(status)
      }
    }
  }
}
