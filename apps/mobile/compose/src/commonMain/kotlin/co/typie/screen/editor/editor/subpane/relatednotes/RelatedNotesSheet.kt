package co.typie.screen.editor.editor.subpane.relatednotes

import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.isFolder
import co.typie.domain.note.NoteEditState
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
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.ext.verticalScroll
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.graphql.type.NoteStatus
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.navigation.Nav
import co.typie.navigation.PlatformBackHandler
import co.typie.result.Result
import co.typie.route.Route
import co.typie.screen.editor.editor.subpane.EditorResizableSheetSurface
import co.typie.screen.editor.editor.subpane.EditorSubPane
import co.typie.screen.editor.editor.subpane.EditorSubPaneLayoutInfo
import co.typie.screen.editor.editor.subpane.resolveResizableSubPaneVisibleAreaMode
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.reorder.reorderableViewport
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private const val RelatedNotesSheetViewModelKeyPrefix = "editor-related-notes"
private val RelatedNotesInitialHeight = 360.dp
private val RelatedNotesMinHeight = 240.dp
private val RelatedNotesDismissThreshold = 128.dp
private val RelatedNotesMinKeyboardVisibleHeight = 240.dp
private val RelatedNotesListBottomContentPadding = 8.dp

@Composable
internal fun RelatedNotesSheet(
  entityId: String,
  maxTopInset: Dp,
  safeBottomInset: Dp,
  trustedImeBottomInset: Dp,
  onDismissStarted: () -> Unit,
  onDismiss: () -> Unit,
  onLayoutInfoChanged: (EditorSubPaneLayoutInfo) -> Unit,
  onLayoutInfoCleared: (EditorSubPane) -> Unit,
  modifier: Modifier = Modifier,
) {
  val keyboardOcclusion = (trustedImeBottomInset - safeBottomInset).coerceAtLeast(0.dp)
  val model =
    viewModel(key = "$RelatedNotesSheetViewModelKeyPrefix:$entityId") {
      RelatedNotesViewModel(entityId)
    }
  val noteEditState = model.noteEditState
  val toast = LocalToast.current

  DisposableEffect(onLayoutInfoCleared) {
    onDispose { onLayoutInfoCleared(EditorSubPane.RelatedNotes) }
  }
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

  suspend fun collapseExpandedNote(): Boolean {
    return noteEditState.collapse(saveContent = ::saveNoteContent, saveColor = ::saveNoteColor)
  }

  EditorResizableSheetSurface(
    initialHeight = RelatedNotesInitialHeight,
    minHeight = RelatedNotesMinHeight,
    dismissThreshold = RelatedNotesDismissThreshold,
    maxTopInset = maxTopInset,
    keyboardOcclusion = keyboardOcclusion,
    minKeyboardVisibleHeight = RelatedNotesMinKeyboardVisibleHeight,
    onDismissStarted = onDismissStarted,
    onDismissed = onDismiss,
    onGeometryChanged = { geometry ->
      onLayoutInfoChanged(
        EditorSubPaneLayoutInfo(
          pane = EditorSubPane.RelatedNotes,
          visibleHeight = geometry.visibleHeight,
          visibleAreaMode =
            resolveResizableSubPaneVisibleAreaMode(
              sheetHeight = geometry.sheetHeight,
              expandedHeight = geometry.expandedHeight,
            ),
        )
      )
    },
    modifier = modifier,
  ) {
    PlatformBackHandler(enabled = true) { dismiss() }

    RelatedNotesSheetContent(
      entityId = entityId,
      safeBottomInset = safeBottomInset,
      keyboardOcclusion = keyboardOcclusion,
      onDismiss = ::dismiss,
      sheetDragHandleModifier = Modifier.sheetDragHandle(),
      model = model,
      noteEditState = noteEditState,
      saveNoteContent = ::saveNoteContent,
      saveNoteColor = ::saveNoteColor,
      collapseExpandedNote = ::collapseExpandedNote,
    )
  }
}

@Composable
private fun RelatedNotesSheetContent(
  entityId: String,
  safeBottomInset: Dp,
  keyboardOcclusion: Dp,
  onDismiss: () -> Unit,
  sheetDragHandleModifier: Modifier,
  model: RelatedNotesViewModel,
  noteEditState: NoteEditState,
  saveNoteContent: suspend (noteId: String, content: String) -> Boolean,
  saveNoteColor: suspend (noteId: String, color: String) -> Boolean,
  collapseExpandedNote: suspend () -> Boolean,
) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val scrollState = rememberScrollState()
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val noteColorOptions = rememberNoteColorOptions()

  suspend fun flushNoteEdits(noteId: String): Boolean {
    return noteEditState.flush(
      noteId = noteId,
      saveContent = saveNoteContent,
      saveColor = saveNoteColor,
    )
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
    if (!SubscriptionService.gate(sheet, GatedAction.CreateNote)) {
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
    if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) {
      return
    }

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
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      SubscriptionService.requestSubscribeSheet(GatedAction.EditNote)
      return
    }

    if (note.color == color) {
      return
    }

    noteEditState.updateColor(noteId = note.id, value = color, save = saveNoteColor)
  }

  suspend fun handleAddEntity(noteId: String, linkedEntityId: String): Boolean {
    if (!flushNoteEdits(noteId)) {
      return false
    }

    return when (val result = model.addNoteEntity(noteId = noteId, entityId = linkedEntityId)) {
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

  suspend fun handleRemoveEntity(
    note: NoteCard_note,
    linkedEntityId: String,
    sceneStatus: NoteStatus,
  ): Boolean {
    if (!flushNoteEdits(note.id)) {
      return false
    }

    val removesCurrentDocument = linkedEntityId == entityId
    if (removesCurrentDocument) {
      model.listState(sceneStatus).markExiting(note)
    }

    return when (val result = model.removeNoteEntity(noteId = note.id, entityId = linkedEntityId)) {
      is Result.Ok -> {
        noteEditState.commitServerSnapshot(result.value)
        if (removesCurrentDocument) {
          noteEditState.clearExpanded(note.id)
          model.refetch()
        }
        true
      }

      is Result.Err,
      is Result.Exception -> {
        if (removesCurrentDocument) {
          model.listState(sceneStatus).remove(note.id)
        }
        toast.show(ToastType.Error, "연결을 해제할 수 없어요.")
        false
      }
    }
  }

  fun presentEntityPicker(note: NoteCard_note, sceneStatus: NoteStatus) {
    scope.launch {
      if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) return@launch

      sheet.present(stops = NoteEntityPickerStops) {
        NoteEntityPickerSheet(
          linkedEntityIds = note.entities.mapTo(mutableSetOf()) { it.noteLinkedEntity_entity.id },
          onAddEntity = { linkedEntityId -> handleAddEntity(note.id, linkedEntityId) },
          onRemoveEntity = { linkedEntityId ->
            handleRemoveEntity(note, linkedEntityId, sceneStatus)
          },
        )
      }
    }
  }

  fun presentLinkedEntityActions(
    note: NoteCard_note,
    linkedEntity: NoteLinkedEntity_entity,
    sceneStatus: NoteStatus,
  ) {
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
          onUnlink = {
            scope.launch {
              if (SubscriptionService.gate(sheet, GatedAction.EditNote)) {
                handleRemoveEntity(note, linkedEntity.id, sceneStatus)
              }
            }
          },
        )
      }
    }
  }

  SheetLayout(
    modifier = Modifier.fillMaxSize(),
    fillHeight = true,
    bodyScroll = false,
    handleModifier = sheetDragHandleModifier,
    includeBottomInset = false,
    padding = SheetPadding(header = PaddingValues(horizontal = 16.dp), body = PaddingValues(0.dp)),
    header = {
      RelatedNotesSheetBar(
        selectedStatus = model.filterStatus,
        onDismiss = onDismiss,
        onFilterSelect = { nextStatus -> scope.launch { handleFilterSelection(nextStatus) } },
        onCreate = { scope.launch { handleCreateNote() } },
        modifier = sheetDragHandleModifier,
      )
    },
  ) {
    Crossfade(
      targetState = model.filterStatus,
      modifier = Modifier.fillMaxSize().padding(bottom = safeBottomInset + keyboardOcclusion),
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
            noteEditState.updateContent(noteId = noteId, value = content, save = saveNoteContent)
          },
          onBlur = { noteId -> scope.launch { flushNoteEdits(noteId) } },
          onToggleStatus = { note -> scope.launch { handleToggleStatus(note, status) } },
          onColorChange = ::handleColorChange,
          onAddEntity = { note -> presentEntityPicker(note, status) },
          onEntityClick = { note, entity -> presentLinkedEntityActions(note, entity, status) },
          onDelete = { note -> scope.launch { handleDeleteNote(note, status) } },
          onMoveNote = { noteId, lowerOrder, upperOrder ->
            model.moveNote(noteId = noteId, lowerOrder = lowerOrder, upperOrder = upperOrder)
          },
        )
      val reorderState = rememberNoteListReorderState(items = listItems, scrollState = scrollState)

      Box(modifier = Modifier.fillMaxSize().reorderableViewport(state = reorderState)) {
        Column(
          modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(horizontal = 16.dp),
          verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          NoteList(
            emptyMessage = status.emptyMessage(),
            queryState = model.queryState(status),
            items = listItems,
            onEnterAnimationFinished = listState::finishEntering,
            onExitAnimationFinished = listState::finishExiting,
            reorderState = reorderState,
            noteColorOptions = noteColorOptions,
            interactive = status == model.filterStatus,
            reorderEnabled = SubscriptionService.entitlement !is Entitlement.Expired,
            contentEditable = SubscriptionService.entitlement !is Entitlement.Expired,
            actions = listActions,
          )

          Spacer(Modifier.height(RelatedNotesListBottomContentPadding))
        }
      }
    }
  }
}

@Composable
private fun RelatedNotesSheetBar(
  selectedStatus: NoteStatus,
  onDismiss: () -> Unit,
  onFilterSelect: (NoteStatus) -> Unit,
  onCreate: () -> Unit,
  modifier: Modifier = Modifier,
) {
  Box(modifier = modifier.fillMaxWidth().height(44.dp).padding(horizontal = 0.dp)) {
    SheetBarButton(
      icon = Lucide.X,
      onClick = { onDismiss() },
      modifier = Modifier.align(Alignment.CenterStart),
    )

    Text(
      text = "노트",
      modifier = Modifier.align(Alignment.Center).padding(horizontal = 104.dp),
      style = AppTheme.typography.title,
      color = AppTheme.colors.textDefault,
      overflow = TextOverflow.Ellipsis,
      maxLines = 1,
    )

    Row(
      modifier = Modifier.align(Alignment.CenterEnd),
      horizontalArrangement = Arrangement.spacedBy(8.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      RelatedNotesFilterPopover(selectedStatus = selectedStatus, onSelect = onFilterSelect)
      SheetBarButton(icon = Typie.StickyNotePlus, onClick = { onCreate() })
    }
  }
}

@Composable
private fun RelatedNotesFilterPopover(selectedStatus: NoteStatus, onSelect: (NoteStatus) -> Unit) {
  PopoverMenu(anchor = { SheetBarButton(icon = Lucide.ListFilter, onClick = {}) }) {
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
        },
        onClick = { onSelect(status) },
      )
    }
  }
}
