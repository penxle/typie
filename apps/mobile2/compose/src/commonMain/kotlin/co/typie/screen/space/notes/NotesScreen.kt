package co.typie.screen.space.notes

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
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
import co.typie.domain.note.rememberNoteColorOptions
import co.typie.ext.navigationBarsPadding
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.graphql.type.NoteStatus
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.navigation.Nav
import co.typie.result.Result
import co.typie.route.Route
import co.typie.shell.MainBottomBarActionButton
import co.typie.shell.MainBottomBarPill
import co.typie.ui.component.Screen
import co.typie.ui.component.SpacePopover
import co.typie.ui.component.SpacePopoverLeadingKey
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private object NotesFilterTopBarTrailingKey

@Composable
fun NotesScreen() {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val model = viewModel { NotesViewModel() }
  val screenState = model.screenState
  val scrollState = rememberScrollState()
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val siteId = model.siteId
  val noteColorOptions = rememberNoteColorOptions()

  LaunchedEffect(Unit) { model.onScreenEntered() }

  LaunchedEffect(model.openQuery.state) {
    if (model.openQuery.state is QueryState.Success<*>) {
      screenState.syncScene(NoteStatus.OPEN, model.settledNotes(NoteStatus.OPEN))
    }
  }

  LaunchedEffect(model.resolvedQuery.state) {
    if (model.resolvedQuery.state is QueryState.Success<*>) {
      screenState.syncScene(NoteStatus.RESOLVED, model.settledNotes(NoteStatus.RESOLVED))
    }
  }

  val activeQuery = model.query(screenState.filterStatus)
  LaunchedEffect(screenState.filterStatus, activeQuery.state) {
    if (activeQuery.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { activeQuery.refetch() })
    }
  }

  DisposableEffect(screenState, model) {
    onDispose {
      screenState.dispose(
        savePendingContent = model::savePendingNoteContent,
        savePendingColor = model::savePendingNoteColor,
      )
    }
  }

  suspend fun saveNoteContent(noteId: String, content: String): Boolean {
    return when (val result = model.updateNoteContent(noteId = noteId, content = content)) {
      is Result.Ok -> {
        screenState.commitServerSnapshot(result.value)
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
        screenState.commitServerSnapshot(result.value)
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
    return screenState.flush(
      noteId = noteId,
      saveContent = ::saveNoteContent,
      saveColor = ::saveNoteColor,
    )
  }

  suspend fun collapseExpandedNote(): Boolean {
    return screenState.collapse(saveContent = ::saveNoteContent, saveColor = ::saveNoteColor)
  }

  suspend fun handleExpandNote(note: NoteCard_note) {
    val expandedNoteId = screenState.expandedNoteId
    if (expandedNoteId != null && expandedNoteId != note.id && !flushNoteEdits(expandedNoteId)) {
      return
    }

    screenState.open(note = note)
  }

  suspend fun handleFilterSelection(nextStatus: NoteStatus) {
    if (nextStatus == screenState.filterStatus || nextStatus == NoteStatus.UNKNOWN__) {
      return
    }

    if (!collapseExpandedNote()) {
      return
    }

    screenState.updateFilterStatus(nextStatus)
    scrollState.scrollTo(0)
  }

  suspend fun handleCreateNote() {
    if (siteId == null) {
      return
    }

    if (!collapseExpandedNote()) {
      return
    }

    if (screenState.filterStatus == NoteStatus.RESOLVED) {
      screenState.updateFilterStatus(NoteStatus.OPEN)
      scrollState.scrollTo(0)
    }

    when (val result = model.createNote()) {
      is Result.Ok -> {
        screenState.sceneState(NoteStatus.OPEN).markEntering(result.value)
        screenState.open(note = result.value)
        model.refetch(NoteStatus.OPEN)
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

    screenState.cancelPendingSaves(note.id)
    screenState.sceneState(sceneStatus).markExiting(note)

    when (model.deleteNote(note.id)) {
      is Result.Ok -> {
        screenState.remove(note.id)
        model.refetch(sceneStatus)
        toast.show(ToastType.Success, "노트를 삭제했어요.")
      }

      is Result.Err,
      is Result.Exception -> {
        screenState.sceneState(sceneStatus).remove(note.id)
        toast.show(ToastType.Error, "노트를 삭제할 수 없어요.")
      }
    }
  }

  suspend fun handleToggleStatus(note: NoteCard_note, sceneStatus: NoteStatus) {
    if (!flushNoteEdits(note.id)) {
      return
    }

    val nextStatus = note.status.toggled()
    screenState.sceneState(sceneStatus).markExiting(note.copy(status = nextStatus))

    when (val result = model.updateNoteStatus(noteId = note.id, status = nextStatus)) {
      is Result.Ok -> {
        screenState.commitServerSnapshot(result.value)
        screenState.clearExpanded(note.id)
        screenState.sceneState(nextStatus).expectEntry(result.value)
        model.refetch(NoteStatus.OPEN)
        model.refetch(NoteStatus.RESOLVED)
      }

      is Result.Err,
      is Result.Exception -> {
        screenState.sceneState(sceneStatus).remove(note.id)
        toast.show(ToastType.Error, "상태를 바꿀 수 없어요.")
      }
    }
  }

  fun handleColorChange(note: NoteCard_note, color: String) {
    if (note.color == color) {
      return
    }

    screenState.updateColor(noteId = note.id, value = color, save = ::saveNoteColor)
  }

  suspend fun handleAddEntity(noteId: String, entityId: String): Boolean {
    if (!flushNoteEdits(noteId)) {
      return false
    }

    return when (val result = model.addNoteEntity(noteId = noteId, entityId = entityId)) {
      is Result.Ok -> {
        screenState.commitServerSnapshot(result.value)
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
        screenState.commitServerSnapshot(result.value)
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
    leadingKey = SpacePopoverLeadingKey,
    leading = { SpacePopover() },
    center = { Text("노트", style = AppTheme.typography.title) },
    trailingKey = NotesFilterTopBarTrailingKey,
    trailing = {
      NotesFilterPopover(
        selectedStatus = screenState.filterStatus,
        onSelect = { nextStatus -> scope.launch { handleFilterSelection(nextStatus) } },
      )
    },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  ProvideBottomBar(
    pill = { MainBottomBarPill() },
    action = {
      MainBottomBarActionButton(
        icon = Typie.StickyNotePlus,
        onClick = { scope.launch { handleCreateNote() } },
      )
    },
  )

  Screen(background = AppTheme.colors.surfaceBase) { contentPadding ->
    Crossfade(
      targetState = screenState.filterStatus,
      modifier = Modifier,
      animationSpec = tween(durationMillis = 200),
    ) { status ->
      val sceneState = screenState.sceneState(status)
      val renderedNotes = sceneState.merge(model.notes(status)).map(screenState::overlay)
      val listItems = renderedNotes.map { note ->
        NoteListItem(
          note = note,
          expanded = screenState.expandedNoteId == note.id,
          isSaving = screenState.isSaving(note.id) || screenState.isSavingColor(note.id),
          hasPendingColor = screenState.hasPendingColor(note.id),
          isDirty = screenState.isDirty(note.id),
          isEntering = sceneState.isEntering(note.id),
          isExiting = sceneState.isExiting(note.id),
          isExitVisible = sceneState.isExitVisible(note.id),
        )
      }
      val listActions =
        NoteListActions(
          onExpand = { note -> scope.launch { handleExpandNote(note) } },
          onCollapse = { scope.launch { collapseExpandedNote() } },
          onContentChange = { noteId, content ->
            screenState.updateContent(noteId = noteId, value = content, save = ::saveNoteContent)
          },
          onBlur = { noteId -> scope.launch { flushNoteEdits(noteId) } },
          onToggleStatus = { note -> scope.launch { handleToggleStatus(note, status) } },
          onColorChange = ::handleColorChange,
          onAddEntity = ::presentEntityPicker,
          onEntityClick = ::presentLinkedEntityActions,
          onDelete = { note -> scope.launch { handleDeleteNote(note, status) } },
          onMoveNote = { noteId, lowerOrder, upperOrder ->
            val result =
              model.moveNote(noteId = noteId, lowerOrder = lowerOrder, upperOrder = upperOrder)
            if (result is Result.Ok) {
              model.refetch(status)
            }
            result
          },
        )

      NoteList(
        emptyMessage = status.emptyMessage(),
        queryState = model.query(status).state,
        items = listItems,
        onEnterAnimationFinished = sceneState::finishEntering,
        onExitAnimationFinished = sceneState::finishExiting,
        scrollState = scrollState,
        contentPadding = contentPadding,
        noteColorOptions = noteColorOptions,
        interactive = status == screenState.filterStatus,
        actions = listActions,
      )
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
              tint = AppTheme.colors.textSecondary,
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
                  tint = AppTheme.colors.brand,
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

internal fun NoteStatus.filterLabel(): String =
  when (this) {
    NoteStatus.OPEN -> "진행 중"
    NoteStatus.RESOLVED -> "완료됨"
    NoteStatus.UNKNOWN__ -> "진행 중"
  }

internal fun NoteStatus.emptyMessage(): String =
  when (this) {
    NoteStatus.OPEN -> "진행 중 노트가 없어요"
    NoteStatus.RESOLVED -> "완료된 노트가 없어요"
    NoteStatus.UNKNOWN__ -> "진행 중 노트가 없어요"
  }

internal fun NoteStatus.toggled(): NoteStatus =
  when (this) {
    NoteStatus.OPEN -> NoteStatus.RESOLVED
    NoteStatus.RESOLVED -> NoteStatus.OPEN
    NoteStatus.UNKNOWN__ -> NoteStatus.OPEN
  }
