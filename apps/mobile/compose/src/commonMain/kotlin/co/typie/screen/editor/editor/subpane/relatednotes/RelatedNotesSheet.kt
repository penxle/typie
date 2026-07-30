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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.isFolder
import co.typie.domain.note.NoteActionOutcome
import co.typie.domain.note.NoteActionRequest
import co.typie.domain.note.NoteActions
import co.typie.domain.note.NoteEditState
import co.typie.domain.note.NoteEditorBringIntoViewScope
import co.typie.domain.note.NoteEntityMutationOutcome
import co.typie.domain.note.NoteEntityPickerSheet
import co.typie.domain.note.NoteEntityPickerStops
import co.typie.domain.note.NoteLinkedEntityActionsSheet
import co.typie.domain.note.NoteList
import co.typie.domain.note.NoteListActions
import co.typie.domain.note.NoteListIdentity
import co.typie.domain.note.NoteSaveOutcome
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
  siteId: String,
  maxTopInset: Dp,
  safeBottomInset: Dp,
  trustedImeBottomInset: Dp,
  onDismissStarted: () -> Unit,
  onDismiss: () -> Unit,
  onLayoutInfoChanged: (EditorSubPaneLayoutInfo) -> Unit,
  onLayoutInfoCleared: (EditorSubPane) -> Unit,
  registerRouteRemovalPreparation: (suspend () -> Boolean) -> (() -> Unit),
  modifier: Modifier = Modifier,
) {
  val keyboardOcclusion = (trustedImeBottomInset - safeBottomInset).coerceAtLeast(0.dp)
  val model =
    viewModel(key = "$RelatedNotesSheetViewModelKeyPrefix:$siteId:$entityId") {
      RelatedNotesViewModel(entityId = entityId, siteId = siteId)
    }
  val noteEditState = model.noteEditState
  val noteActions = remember { NoteActions() }
  SideEffect {
    noteActions.activate(
      siteId = siteId,
      entityId = entityId,
      editState = noteEditState,
      onTerminal = model::convergeDeletedNote,
    )
  }

  DisposableEffect(noteEditState, model, registerRouteRemovalPreparation) {
    val unregister = registerRouteRemovalPreparation {
      noteEditState.flushPendingEdits(
        savePendingContent = model::savePendingNoteContent,
        savePendingColor = model::savePendingNoteColor,
      )
    }
    onDispose { unregister() }
  }

  DisposableEffect(onLayoutInfoCleared) {
    onDispose { onLayoutInfoCleared(EditorSubPane.RelatedNotes) }
  }
  DisposableEffect(noteEditState, model) {
    onDispose {
      noteActions.dispose()
      noteEditState.dispose(
        savePendingContent = model::savePendingNoteContent,
        savePendingColor = model::savePendingNoteColor,
      )
    }
  }

  suspend fun saveNoteContent(noteId: String, content: String): NoteSaveOutcome {
    val request =
      noteActions.captureRequest(siteId = siteId, entityId = entityId)
        ?: return NoteSaveOutcome.Superseded
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      SubscriptionService.requestSubscribeSheet(GatedAction.EditNote)
      return NoteSaveOutcome.SubscriptionGated
    }

    return noteActions.save(request, noteId) { _ ->
      model.updateNoteContent(noteId = noteId, content = content)
    }
  }

  suspend fun saveNoteColor(noteId: String, color: String): NoteSaveOutcome {
    val request =
      noteActions.captureRequest(siteId = siteId, entityId = entityId)
        ?: return NoteSaveOutcome.Superseded
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      SubscriptionService.requestSubscribeSheet(GatedAction.EditNote)
      return NoteSaveOutcome.SubscriptionGated
    }

    return noteActions.save(request, noteId) { _ ->
      model.updateNoteColor(noteId = noteId, color = color)
    }
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
      noteActions = noteActions,
      saveNoteContent = ::saveNoteContent,
      saveNoteColor = ::saveNoteColor,
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
  noteActions: NoteActions,
  saveNoteContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  saveNoteColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
) {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val scrollState = rememberScrollState()
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val noteColorOptions = rememberNoteColorOptions()

  LaunchedEffect(noteEditState, toast) {
    noteEditState.saveFailures.collect { toast.show(ToastType.Error, "노트를 저장하지 못했어요.") }
  }

  suspend fun handleExpandNote(note: NoteCard_note) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = entityId) ?: return
    noteActions.open(
      request = request,
      note = note,
      saveContent = saveNoteContent,
      saveColor = saveNoteColor,
    )
  }

  suspend fun handleFilterSelection(request: NoteActionRequest, nextStatus: NoteStatus) {
    if (!noteActions.isCurrent(request)) return
    if (nextStatus == model.filterStatus || nextStatus == NoteStatus.UNKNOWN__) {
      return
    }

    if (
      !noteActions.collapse(
        request = request,
        saveContent = saveNoteContent,
        saveColor = saveNoteColor,
      )
    ) {
      return
    }

    scrollState.scrollTo(0)
    if (!noteActions.isCurrent(request)) return
    model.updateFilterStatus(nextStatus)
  }

  suspend fun handleCreateNote(request: NoteActionRequest, autoFocusContent: Boolean = false) {
    if (!noteActions.isCurrent(request)) return
    if (!SubscriptionService.gate(sheet, GatedAction.CreateNote)) {
      return
    }
    if (!noteActions.isCurrent(request)) return

    if (
      !noteActions.collapse(
        request = request,
        saveContent = saveNoteContent,
        saveColor = saveNoteColor,
      )
    ) {
      return
    }

    when (val outcome = noteActions.create(request) { model.createNote() }) {
      is NoteActionOutcome.Success -> {
        if (model.filterStatus == NoteStatus.RESOLVED) {
          model.updateFilterStatus(NoteStatus.OPEN)
        }
        model.listState(NoteStatus.OPEN).markEntering(outcome.value)
        noteEditState.open(note = outcome.value, autoFocusContent = autoFocusContent)
        scrollState.animateScrollTo(0)
      }

      is NoteActionOutcome.Failure -> {
        toast.show(ToastType.Error, "노트를 만들지 못했어요.")
      }

      null,
      NoteActionOutcome.Terminal,
      NoteActionOutcome.Superseded -> Unit
    }
  }

  fun handleShortcutCreateNote() {
    val request = noteActions.captureRequest() ?: return
    scope.launch { handleCreateNote(request = request, autoFocusContent = true) }
  }

  suspend fun handleDeleteNote(note: NoteCard_note) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = entityId) ?: return
    if (noteActions.isPendingDeletion(note.id)) return

    if (note.content.isNotBlank()) {
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
    }
    if (!noteActions.isCurrent(request)) return

    val outcome =
      noteActions.delete(request = request, noteId = note.id) { model.deleteNote(note.id) }
    if (outcome is NoteActionOutcome.Failure) {
      toast.show(ToastType.Error, "노트를 삭제하지 못했어요.")
    }
  }

  suspend fun handleToggleStatus(note: NoteCard_note, sceneStatus: NoteStatus) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = entityId) ?: return
    val nextStatus = note.status.toggled()
    val outcome =
      noteActions.toggleStatus(
        request = request,
        note = note,
        sourceState = model.listState(sceneStatus),
        destinationState = model.listState(nextStatus),
        beforeMutation = beforeMutation@{
            if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) {
              return@beforeMutation false
            }
            if (!noteActions.isCurrent(request)) return@beforeMutation false
            noteActions.flush(
              request = request,
              noteId = note.id,
              saveContent = saveNoteContent,
              saveColor = saveNoteColor,
            )
          },
      ) { _, status ->
        model.updateNoteStatus(noteId = note.id, status = status)
      }
    if (outcome is NoteActionOutcome.Failure) {
      toast.show(ToastType.Error, "상태를 바꾸지 못했어요.")
    }
  }

  fun handleColorChange(note: NoteCard_note, color: String) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = entityId) ?: return
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      SubscriptionService.requestSubscribeSheet(GatedAction.EditNote)
      return
    }

    if (note.color == color) {
      return
    }

    noteActions.updateColor(
      request = request,
      noteId = note.id,
      value = color,
      save = saveNoteColor,
    )
  }

  suspend fun handleAddEntity(
    request: NoteActionRequest,
    noteId: String,
    linkedEntityId: String,
  ): NoteEntityMutationOutcome {
    if (!noteActions.isCurrent(request)) return NoteEntityMutationOutcome.NotUpdated
    if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) {
      return NoteEntityMutationOutcome.NotUpdated
    }
    if (!noteActions.isCurrent(request)) return NoteEntityMutationOutcome.NotUpdated

    if (
      !noteActions.flush(
        request = request,
        noteId = noteId,
        saveContent = saveNoteContent,
        saveColor = saveNoteColor,
      )
    ) {
      return NoteEntityMutationOutcome.NotUpdated
    }

    return when (
      val outcome =
        noteActions.update(request = request, noteId = noteId) {
          model.addNoteEntity(noteId = noteId, entityId = linkedEntityId)
        }
    ) {
      is NoteActionOutcome.Success -> NoteEntityMutationOutcome.Updated
      NoteActionOutcome.Terminal -> NoteEntityMutationOutcome.Terminal
      NoteActionOutcome.Superseded -> NoteEntityMutationOutcome.NotUpdated
      is NoteActionOutcome.Failure -> {
        toast.show(ToastType.Error, "연결을 추가하지 못했어요.")
        NoteEntityMutationOutcome.NotUpdated
      }
    }
  }

  suspend fun handleRemoveEntity(
    request: NoteActionRequest,
    note: NoteCard_note,
    linkedEntityId: String,
    sceneStatus: NoteStatus,
  ): NoteEntityMutationOutcome {
    if (!noteActions.isCurrent(request)) return NoteEntityMutationOutcome.NotUpdated
    if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) {
      return NoteEntityMutationOutcome.NotUpdated
    }
    if (!noteActions.isCurrent(request)) return NoteEntityMutationOutcome.NotUpdated

    if (
      !noteActions.flush(
        request = request,
        noteId = note.id,
        saveContent = saveNoteContent,
        saveColor = saveNoteColor,
      )
    ) {
      return NoteEntityMutationOutcome.NotUpdated
    }

    val removesCurrentDocument = linkedEntityId == entityId
    val outcome =
      if (removesCurrentDocument) {
        noteActions.updateAndExit(
          request = request,
          note = note,
          state = model.listState(sceneStatus),
        ) {
          model.removeNoteEntity(noteId = note.id, entityId = linkedEntityId)
        }
      } else {
        noteActions.update(request = request, noteId = note.id) {
          model.removeNoteEntity(noteId = note.id, entityId = linkedEntityId)
        }
      }
    return when (outcome) {
      is NoteActionOutcome.Success -> {
        if (removesCurrentDocument) {
          noteEditState.clearExpanded(siteId = request.siteId, noteId = note.id)
        }
        NoteEntityMutationOutcome.Updated
      }

      NoteActionOutcome.Terminal -> NoteEntityMutationOutcome.Terminal

      NoteActionOutcome.Superseded -> NoteEntityMutationOutcome.NotUpdated

      is NoteActionOutcome.Failure -> {
        toast.show(ToastType.Error, "연결을 해제하지 못했어요.")
        NoteEntityMutationOutcome.NotUpdated
      }
    }
  }

  fun presentEntityPicker(note: NoteCard_note, sceneStatus: NoteStatus) {
    val request = noteActions.captureRequest() ?: return
    scope.launch {
      if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) return@launch
      if (!noteActions.isCurrent(request)) return@launch

      sheet.present(stops = NoteEntityPickerStops) {
        NoteEntityPickerSheet(
          siteId = model.siteId,
          linkedEntityIds = note.entities.mapTo(mutableSetOf()) { it.noteLinkedEntity_entity.id },
          onAddEntity = { linkedEntityId -> handleAddEntity(request, note.id, linkedEntityId) },
          onRemoveEntity = { linkedEntityId ->
            handleRemoveEntity(request, note, linkedEntityId, sceneStatus)
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
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = entityId) ?: return
    scope.launch {
      if (!noteActions.isCurrent(request)) return@launch
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
            scope.launch { handleRemoveEntity(request, note, linkedEntity.id, sceneStatus) }
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
        onFilterSelect = { nextStatus ->
          noteActions.captureRequest()?.let { request ->
            scope.launch { handleFilterSelection(request = request, nextStatus = nextStatus) }
          }
        },
        onCreate = {
          noteActions.captureRequest()?.let { request ->
            scope.launch { handleCreateNote(request = request) }
          }
        },
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
      val authoritativeNotes = model.notes(status)
      val renderedNotes = listState.merge(authoritativeNotes).map(noteEditState::overlay)
      val listItems =
        noteActions.listItems(notes = renderedNotes, state = listState, editState = noteEditState)
      val listActions =
        NoteListActions(
          onExpand = { note -> scope.launch { handleExpandNote(note) } },
          onCollapse = {
            noteActions.captureRequest()?.let { request ->
              scope.launch {
                noteActions.collapse(
                  request = request,
                  saveContent = saveNoteContent,
                  saveColor = saveNoteColor,
                )
              }
            }
          },
          onCreateNote = ::handleShortcutCreateNote,
          onContentChange = { note, content ->
            noteActions.captureRequest(siteId = note.site.id, entityId = entityId)?.let { request ->
              noteActions.updateContent(
                request = request,
                noteId = note.id,
                value = content,
                save = saveNoteContent,
              )
            }
          },
          onBlur = { note ->
            noteActions.captureRequest(siteId = note.site.id, entityId = entityId)?.let { request ->
              scope.launch {
                noteActions.flush(
                  request = request,
                  noteId = note.id,
                  saveContent = saveNoteContent,
                  saveColor = saveNoteColor,
                )
              }
            }
          },
          onToggleStatus = { note -> scope.launch { handleToggleStatus(note, status) } },
          onColorChange = ::handleColorChange,
          onAddEntity = { note -> presentEntityPicker(note, status) },
          onEntityClick = { note, entity -> presentLinkedEntityActions(note, entity, status) },
          onDelete = { note -> scope.launch { handleDeleteNote(note) } },
          onMoveNote = { note, lowerOrder, upperOrder ->
            model.moveNote(note = note, lowerOrder = lowerOrder, upperOrder = upperOrder)
          },
        )
      val reorderState = rememberNoteListReorderState(items = listItems, scrollState = scrollState)

      NoteEditorBringIntoViewScope {
        Box(modifier = Modifier.fillMaxSize().reorderableViewport(state = reorderState)) {
          Column(
            modifier =
              Modifier.fillMaxSize().verticalScroll(scrollState).padding(horizontal = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
          ) {
            NoteList(
              identity =
                NoteListIdentity(siteId = model.siteId, status = status, entityId = entityId),
              emptyMessage = status.emptyMessage(),
              queryState = model.queryState(status),
              items = listItems,
              authoritativeNotes = authoritativeNotes,
              onEnterAnimationFinished = listState::finishEntering,
              onExitAnimationFinished = listState::finishExiting,
              reorderState = reorderState,
              noteColorOptions = noteColorOptions,
              interactive = status == model.filterStatus,
              onRetry = model::refetch,
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
