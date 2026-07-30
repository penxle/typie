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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.entity.isFolder
import co.typie.domain.note.NoteActionOutcome
import co.typie.domain.note.NoteActionRequest
import co.typie.domain.note.NoteActions
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
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.ext.safeDrawing
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.graphql.type.NoteStatus
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.navigation.Nav
import co.typie.navigation.RouteRemovalDecision
import co.typie.navigation.RouteRemovalInterceptor
import co.typie.navigation.RouteRemovalPreparation
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
  val noteActions = remember { NoteActions() }
  SideEffect {
    noteActions.activate(
      siteId = siteId,
      entityId = null,
      editState = noteEditState,
      onTerminal = model::convergeDeletedNote,
    )
  }
  val noteColorOptions = rememberNoteColorOptions()
  val routeRemovalInterceptor =
    remember(noteEditState, model, dialog) {
      object : RouteRemovalInterceptor {
        override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation =
          if (
            noteEditState.flushPendingEdits(
              savePendingContent = model::savePendingNoteContent,
              savePendingColor = model::savePendingNoteColor,
            )
          ) {
            RouteRemovalPreparation.Ready
          } else {
            RouteRemovalPreparation.NeedsDecision
          }

        override suspend fun resolveDecision(): RouteRemovalDecision {
          val result =
            dialog.confirm(
              title = "저장을 완료하지 못했어요",
              message = "지금 닫으면 최근 변경사항을 잃을 수 있어요.",
              confirmText = "저장하지 않고 닫기",
              cancelText = "계속 편집",
              confirmIsDestructive = true,
            )
          return if (result is DialogResult.Resolved) {
            RouteRemovalDecision.ProceedWithRemoval
          } else {
            RouteRemovalDecision.CancelRemoval
          }
        }

        override suspend fun rollback() = Unit
      }
    }

  DisposableEffect(nav, routeRemovalInterceptor) {
    val unregister = nav.routeRemovals.register(Route.Notes, routeRemovalInterceptor)
    onDispose { unregister() }
  }

  LaunchedEffect(model) {
    if (model.query.state !is QueryState.Loading) {
      model.refetch()
    }
  }

  LaunchedEffect(noteEditState, toast) {
    noteEditState.saveFailures.collect { toast.show(ToastType.Error, "노트를 저장하지 못했어요.") }
  }

  LaunchedEffect(siteId, noteEditState, model) {
    val activeNoteId = noteEditState.expandedNoteId ?: return@LaunchedEffect
    val activeSiteId = noteEditState.expandedNoteSiteId ?: return@LaunchedEffect
    if (activeSiteId != siteId) {
      noteEditState.dispose(
        savePendingContent = model::savePendingNoteContent,
        savePendingColor = model::savePendingNoteColor,
      )
      noteEditState.remove(siteId = activeSiteId, noteId = activeNoteId)
    }
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
    val activeSiteId = siteId ?: return NoteSaveOutcome.Superseded
    val request =
      noteActions.captureRequest(siteId = activeSiteId, entityId = null)
        ?: return NoteSaveOutcome.Superseded
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      SubscriptionService.requestSubscribeSheet(GatedAction.EditNote)
      return NoteSaveOutcome.SubscriptionGated
    }

    return noteActions.save(request, noteId) { requestSiteId ->
      model.updateNoteContent(siteId = requestSiteId, noteId = noteId, content = content)
    }
  }

  suspend fun saveNoteColor(noteId: String, color: String): NoteSaveOutcome {
    val activeSiteId = siteId ?: return NoteSaveOutcome.Superseded
    val request =
      noteActions.captureRequest(siteId = activeSiteId, entityId = null)
        ?: return NoteSaveOutcome.Superseded
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      SubscriptionService.requestSubscribeSheet(GatedAction.EditNote)
      return NoteSaveOutcome.SubscriptionGated
    }

    return noteActions.save(request, noteId) { requestSiteId ->
      model.updateNoteColor(siteId = requestSiteId, noteId = noteId, color = color)
    }
  }

  suspend fun handleExpandNote(note: NoteCard_note) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = null) ?: return
    noteActions.open(
      request = request,
      note = note,
      saveContent = ::saveNoteContent,
      saveColor = ::saveNoteColor,
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
        saveContent = ::saveNoteContent,
        saveColor = ::saveNoteColor,
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
        saveContent = ::saveNoteContent,
        saveColor = ::saveNoteColor,
      )
    ) {
      return
    }

    when (
      val outcome =
        noteActions.create(request) { activeSiteId -> model.createNote(siteId = activeSiteId) }
    ) {
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
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = null) ?: return
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
      noteActions.delete(request = request, noteId = note.id) { activeSiteId ->
        model.deleteNote(siteId = activeSiteId, noteId = note.id)
      }
    if (outcome is NoteActionOutcome.Failure) {
      toast.show(ToastType.Error, "노트를 삭제하지 못했어요.")
    }
  }

  suspend fun handleToggleStatus(note: NoteCard_note, sceneStatus: NoteStatus) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = null) ?: return
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
              saveContent = ::saveNoteContent,
              saveColor = ::saveNoteColor,
            )
          },
      ) { activeSiteId, status ->
        model.updateNoteStatus(siteId = activeSiteId, noteId = note.id, status = status)
      }
    if (outcome is NoteActionOutcome.Failure) {
      toast.show(ToastType.Error, "상태를 바꾸지 못했어요.")
    }
  }

  fun handleColorChange(note: NoteCard_note, color: String) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = null) ?: return
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
      save = ::saveNoteColor,
    )
  }

  suspend fun handleAddEntity(
    request: NoteActionRequest,
    noteId: String,
    entityId: String,
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
        saveContent = ::saveNoteContent,
        saveColor = ::saveNoteColor,
      )
    ) {
      return NoteEntityMutationOutcome.NotUpdated
    }

    return when (
      val outcome =
        noteActions.update(request = request, noteId = noteId) { activeSiteId ->
          model.addNoteEntity(siteId = activeSiteId, noteId = noteId, entityId = entityId)
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
    noteId: String,
    entityId: String,
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
        saveContent = ::saveNoteContent,
        saveColor = ::saveNoteColor,
      )
    ) {
      return NoteEntityMutationOutcome.NotUpdated
    }

    return when (
      val outcome =
        noteActions.update(request = request, noteId = noteId) { activeSiteId ->
          model.removeNoteEntity(siteId = activeSiteId, noteId = noteId, entityId = entityId)
        }
    ) {
      is NoteActionOutcome.Success -> NoteEntityMutationOutcome.Updated
      NoteActionOutcome.Terminal -> NoteEntityMutationOutcome.Terminal
      NoteActionOutcome.Superseded -> NoteEntityMutationOutcome.NotUpdated
      is NoteActionOutcome.Failure -> {
        toast.show(ToastType.Error, "연결을 해제하지 못했어요.")
        NoteEntityMutationOutcome.NotUpdated
      }
    }
  }

  fun presentEntityPicker(note: NoteCard_note) {
    val activeSiteId = siteId ?: return
    val request = noteActions.captureRequest(siteId = activeSiteId, entityId = null) ?: return

    scope.launch {
      if (!SubscriptionService.gate(sheet, GatedAction.EditNote)) return@launch
      if (!noteActions.isCurrent(request)) return@launch

      sheet.present(stops = NoteEntityPickerStops) {
        NoteEntityPickerSheet(
          siteId = activeSiteId,
          linkedEntityIds = note.entities.mapTo(mutableSetOf()) { it.noteLinkedEntity_entity.id },
          onAddEntity = { entityId -> handleAddEntity(request, note.id, entityId) },
          onRemoveEntity = { entityId -> handleRemoveEntity(request, note.id, entityId) },
        )
      }
    }
  }

  fun presentLinkedEntityActions(note: NoteCard_note, linkedEntity: NoteLinkedEntity_entity) {
    val request = noteActions.captureRequest(siteId = note.site.id, entityId = null) ?: return
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
          onUnlink = { scope.launch { handleRemoveEntity(request, note.id, linkedEntity.id) } },
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
        onSelect = { nextStatus ->
          noteActions.captureRequest()?.let { request ->
            scope.launch { handleFilterSelection(request = request, nextStatus = nextStatus) }
          }
        },
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
        onClick = {
          noteActions.captureRequest()?.let { request ->
            scope.launch { handleCreateNote(request = request) }
          }
        },
      ),
  )

  Screen(background = AppTheme.colors.surfaceCanvas) { innerPadding ->
    Crossfade(
      targetState = model.filterStatus,
      modifier = Modifier,
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
                  saveContent = ::saveNoteContent,
                  saveColor = ::saveNoteColor,
                )
              }
            }
          },
          onCreateNote = ::handleShortcutCreateNote,
          onContentChange = { note, content ->
            noteActions.captureRequest(siteId = note.site.id, entityId = null)?.let { request ->
              noteActions.updateContent(
                request = request,
                noteId = note.id,
                value = content,
                save = ::saveNoteContent,
              )
            }
          },
          onBlur = { note ->
            noteActions.captureRequest(siteId = note.site.id, entityId = null)?.let { request ->
              scope.launch {
                noteActions.flush(
                  request = request,
                  noteId = note.id,
                  saveContent = ::saveNoteContent,
                  saveColor = ::saveNoteColor,
                )
              }
            }
          },
          onToggleStatus = { note -> scope.launch { handleToggleStatus(note, status) } },
          onColorChange = ::handleColorChange,
          onAddEntity = ::presentEntityPicker,
          onEntityClick = ::presentLinkedEntityActions,
          onDelete = { note -> scope.launch { handleDeleteNote(note) } },
          onMoveNote = { note, lowerOrder, upperOrder ->
            model.moveNote(note = note, lowerOrder = lowerOrder, upperOrder = upperOrder)
          },
        )
      val reorderState = rememberNoteListReorderState(items = listItems, scrollState = scrollState)
      val reorderViewportBottomInset =
        WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding() + 72.dp

      NoteEditorBringIntoViewScope {
        Box(
          modifier =
            Modifier.fillMaxSize()
              .reorderableViewport(
                state = reorderState,
                viewportTopInset = topBarOcclusion,
                viewportBottomInset = reorderViewportBottomInset,
              )
              .imePadding()
        ) {
          Column(
            modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(innerPadding),
            verticalArrangement = Arrangement.spacedBy(16.dp),
          ) {
            Skeleton.Keep { Text(text = "노트", style = AppTheme.typography.display) }

            NoteList(
              identity = NoteListIdentity(siteId = siteId.orEmpty(), status = status),
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

            Spacer(Modifier.height(140.dp))
          }
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
