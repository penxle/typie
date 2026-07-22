package co.typie.screen.editor.editor.subpane.comments

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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.StableSelection
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.CommentsSheetComment_comment
import co.typie.graphql.fragment.CommentsSheetThread_thread
import co.typie.graphql.type.UserRole
import co.typie.icons.Lucide
import co.typie.navigation.PlatformBackHandler
import co.typie.result.Result
import co.typie.screen.editor.editor.subpane.EditorResizableSheetSurface
import co.typie.screen.editor.editor.subpane.EditorSubPane
import co.typie.screen.editor.editor.subpane.EditorSubPaneLayoutInfo
import co.typie.screen.editor.editor.subpane.resolveResizableSubPaneVisibleAreaMode
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private val CommentsInitialHeight = 360.dp
private val CommentsMinHeight = 240.dp
private val CommentsDismissThreshold = 128.dp
private val CommentsMinKeyboardVisibleHeight = 240.dp
private val CommentsListBottomContentPadding = 8.dp

@Composable
internal fun CommentsSheet(
  model: CommentsViewModel,
  myId: String?,
  myRole: UserRole?,
  isOwner: Boolean,
  pendingRequest: PendingCommentSheetRequest?,
  onPendingRequestConsumed: (PendingCommentSheetRequest) -> Unit,
  threadLocationById: Map<String, CommentThreadLocation>,
  composeLocation: CommentThreadLocation?,
  createEnabled: Boolean,
  onFreezeCurrentSelection: suspend () -> StableSelection?,
  onInputFocusChanged: (Boolean) -> Unit,
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
  val state = model.threadState
  val dialog = LocalDialog.current
  val latestOnLayoutInfoCleared = rememberUpdatedState(onLayoutInfoCleared)

  DisposableEffect(Unit) { onDispose { latestOnLayoutInfoCleared.value(EditorSubPane.Comments) } }

  EditorResizableSheetSurface(
    initialHeight = CommentsInitialHeight,
    minHeight = CommentsMinHeight,
    dismissThreshold = CommentsDismissThreshold,
    maxTopInset = maxTopInset,
    keyboardOcclusion = keyboardOcclusion,
    minKeyboardVisibleHeight = CommentsMinKeyboardVisibleHeight,
    canDismiss = { confirmDiscardCommentInput(state = state, dialog = dialog) },
    onDismissStarted = onDismissStarted,
    onDismissed = {
      state.activateThread(null)
      onDismiss()
    },
    onGeometryChanged = { geometry ->
      onLayoutInfoChanged(
        EditorSubPaneLayoutInfo(
          pane = EditorSubPane.Comments,
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

    CommentsSheetContent(
      myId = myId,
      myRole = myRole,
      isOwner = isOwner,
      safeBottomInset = safeBottomInset,
      keyboardOcclusion = keyboardOcclusion,
      pendingRequest = pendingRequest,
      onPendingRequestConsumed = onPendingRequestConsumed,
      threadLocationById = threadLocationById,
      composeLocation = composeLocation,
      createEnabled = createEnabled,
      onFreezeCurrentSelection = onFreezeCurrentSelection,
      onInputFocusChanged = onInputFocusChanged,
      onDismiss = ::dismiss,
      sheetDragHandleModifier = Modifier.sheetDragHandle(),
      model = model,
    )
  }
}

@Composable
private fun CommentsSheetContent(
  myId: String?,
  myRole: UserRole?,
  isOwner: Boolean,
  safeBottomInset: Dp,
  keyboardOcclusion: Dp,
  pendingRequest: PendingCommentSheetRequest?,
  onPendingRequestConsumed: (PendingCommentSheetRequest) -> Unit,
  threadLocationById: Map<String, CommentThreadLocation>,
  composeLocation: CommentThreadLocation?,
  createEnabled: Boolean,
  onFreezeCurrentSelection: suspend () -> StableSelection?,
  onInputFocusChanged: (Boolean) -> Unit,
  onDismiss: () -> Unit,
  sheetDragHandleModifier: Modifier,
  model: CommentsViewModel,
) {
  val state = model.threadState
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  var pendingReopenedThreadId by remember { mutableStateOf<String?>(null) }
  var commentSubmitInProgress by remember { mutableStateOf(false) }

  suspend fun afterDiscardingCommentInput(block: suspend () -> Unit) {
    if (confirmDiscardCommentInput(state = state, dialog = dialog)) {
      block()
    }
  }

  suspend fun submitCommentDraft(block: suspend () -> Unit) {
    if (commentSubmitInProgress) {
      return
    }
    commentSubmitInProgress = true
    try {
      block()
    } finally {
      commentSubmitInProgress = false
    }
  }

  suspend fun selectFilter(filter: CommentFilter) {
    if (filter == state.filter) {
      return
    }
    afterDiscardingCommentInput {
      model.updateFilter(filter)
      state.activateThread(null)
      scrollState.scrollTo(0)
    }
  }

  suspend fun createVirtualThread(selection: StableSelection?) {
    if (selection == null) {
      toast.show(ToastType.Error, "선택 영역에 코멘트를 달 수 없어요.")
      return
    }

    if (state.virtualThread?.selection?.sameRangeAs(selection) == true) {
      model.updateFilter(CommentFilter.Open)
      withFrameNanos {}
      scrollState.scrollTo(scrollState.maxValue)
      return
    }

    val lookup = model.openThreadLookupForSelection(selection)
    if (lookup == OpenThreadSelectionLookup.Unavailable) {
      toast.show(ToastType.Error, "기존 코멘트를 확인할 수 없어요.")
      return
    }

    afterDiscardingCommentInput {
      when (lookup) {
        is OpenThreadSelectionLookup.Found -> {
          model.updateFilter(CommentFilter.Open)
          state.clearVirtualThread()
          state.activateThread(lookup.threadId)
          return@afterDiscardingCommentInput
        }
        OpenThreadSelectionLookup.NotFound -> Unit
        OpenThreadSelectionLookup.Unavailable -> Unit
      }

      if (state.virtualThread != null) {
        state.clearVirtualThread()
      }
      model.updateFilter(CommentFilter.Open)
      state.createVirtualThread(selection)
      withFrameNanos {}
      scrollState.scrollTo(scrollState.maxValue)
    }
  }

  suspend fun activateThreadById(threadId: String?) {
    afterDiscardingCommentInput {
      model.updateFilter(CommentFilter.Open)
      state.clearVirtualThread()
      state.activateThread(threadId)
    }
  }

  suspend fun activateThread(thread: CommentsSheetThread_thread) {
    afterDiscardingCommentInput { state.activateThread(thread.id) }
  }

  suspend fun submitReply(threadId: String, content: String) {
    val trimmedContent = content.trim()
    if (trimmedContent.isEmpty()) {
      return
    }
    submitCommentDraft {
      when (model.createComment(threadId = threadId, content = trimmedContent)) {
        is Result.Ok -> state.clearReplyText()
        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "코멘트를 작성할 수 없어요.")
      }
    }
  }

  suspend fun submitVirtual(content: String) {
    val virtualThread = state.virtualThread ?: return
    val trimmedContent = content.trim()
    if (trimmedContent.isEmpty()) {
      return
    }
    submitCommentDraft {
      when (
        val result =
          model.createThread(selection = virtualThread.selection, content = trimmedContent)
      ) {
        is Result.Ok -> {
          state.clearVirtualThread()
          state.activateThread(result.value.id)
        }

        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "코멘트를 작성할 수 없어요.")
      }
    }
  }

  suspend fun resolveThread(thread: CommentsSheetThread_thread) {
    afterDiscardingCommentInput {
      when (model.resolveThread(thread.id)) {
        is Result.Ok -> toast.show(ToastType.Success, "코멘트를 해결했어요.")
        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "상태를 바꿀 수 없어요.")
      }
    }
  }

  suspend fun unresolveThread(thread: CommentsSheetThread_thread) {
    afterDiscardingCommentInput {
      when (val result = model.unresolveThread(thread.id)) {
        is Result.Ok -> {
          pendingReopenedThreadId = result.value.id
          toast.show(ToastType.Success, "코멘트를 다시 열었어요.")
        }

        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "상태를 바꿀 수 없어요.")
      }
    }
  }

  suspend fun deleteThread(thread: CommentsSheetThread_thread) {
    val result =
      dialog.confirm(
        title = "코멘트 스레드 삭제",
        message = "이 코멘트 스레드를 삭제하시겠어요? 되돌릴 수 없어요.",
        confirmText = "삭제",
        confirmIsDestructive = true,
      )
    if (result !is DialogResult.Resolved) {
      return
    }
    afterDiscardingCommentInput {
      when (model.deleteThread(thread.id)) {
        is Result.Ok -> toast.show(ToastType.Success, "코멘트를 삭제했어요.")
        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "코멘트를 삭제할 수 없어요.")
      }
    }
  }

  suspend fun startEdit(comment: CommentsSheetComment_comment) {
    afterDiscardingCommentInput { state.startEditing(comment.id, comment.content) }
  }

  suspend fun submitEdit(comment: CommentsSheetComment_comment) {
    val content = state.editingText.trim()
    if (content.isEmpty()) {
      toast.show(ToastType.Error, "내용을 입력해주세요.")
      return
    }
    if (content == comment.content) {
      state.clearEditing()
      return
    }
    submitCommentDraft {
      when (model.updateComment(commentId = comment.id, content = content)) {
        is Result.Ok -> state.clearEditing()
        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "코멘트를 수정할 수 없어요.")
      }
    }
  }

  suspend fun cancelEdit() {
    if (!state.hasDirtyEdit) {
      state.clearEditing()
      return
    }
    val result =
      dialog.confirm(
        title = "수정 취소",
        message = "수정 중인 내용이 사라집니다. 취소할까요?",
        confirmText = "수정 취소",
        confirmIsDestructive = true,
      )
    if (result is DialogResult.Resolved) {
      state.clearEditing()
    }
  }

  suspend fun deleteComment(comment: CommentsSheetComment_comment) {
    val result =
      dialog.confirm(
        title = "코멘트 삭제",
        message = "코멘트를 삭제하시겠어요?",
        confirmText = "삭제",
        confirmIsDestructive = true,
      )
    if (result !is DialogResult.Resolved) {
      return
    }
    afterDiscardingCommentInput {
      when (model.deleteComment(comment.id)) {
        is Result.Ok -> toast.show(ToastType.Success, "코멘트를 삭제했어요.")
        is Result.Err,
        is Result.Exception -> toast.show(ToastType.Error, "코멘트를 삭제할 수 없어요.")
      }
    }
  }

  LaunchedEffect(pendingReopenedThreadId, model.threads(CommentFilter.Open)) {
    val threadId = pendingReopenedThreadId ?: return@LaunchedEffect
    val openThreadExists = model.threads(CommentFilter.Open).any { it.id == threadId }
    if (openThreadExists) {
      state.activateThread(threadId)
      pendingReopenedThreadId = null
    }
  }

  LaunchedEffect(pendingRequest) {
    val request = pendingRequest ?: return@LaunchedEffect
    when (val intent = request.intent) {
      is CommentSheetRequest.Create -> createVirtualThread(intent.selection)
      is CommentSheetRequest.ActivateThread -> activateThreadById(intent.threadId)
      CommentSheetRequest.DiscardVirtualThread -> {
        afterDiscardingCommentInput {}
      }
    }
    onPendingRequestConsumed(request)
  }

  SheetLayout(
    modifier = Modifier.fillMaxSize(),
    fillHeight = true,
    bodyScroll = false,
    handleModifier = sheetDragHandleModifier,
    includeBottomInset = false,
    padding = SheetPadding(header = PaddingValues(horizontal = 16.dp), body = PaddingValues(0.dp)),
    header = {
      CommentsSheetBar(
        selectedFilter = state.filter,
        count = model.threads(state.filter).size,
        createEnabled = createEnabled,
        onDismiss = onDismiss,
        onFilterSelect = { filter -> scope.launch { selectFilter(filter) } },
        onCreate = { scope.launch { createVirtualThread(onFreezeCurrentSelection()) } },
        modifier = sheetDragHandleModifier,
      )
    },
  ) {
    Crossfade(
      targetState = state.filter,
      modifier = Modifier.fillMaxSize().padding(bottom = safeBottomInset + keyboardOcclusion),
      animationSpec = tween(durationMillis = 200),
    ) { filter ->
      val threads = model.threads(filter)
      Column(
        modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(horizontal = 16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
      ) {
        when (val queryState = model.queryState(filter)) {
          is QueryState.Error -> MessageRow(text = "코멘트를 불러올 수 없어요.", muted = true)
          QueryState.Loading ->
            if (threads.isEmpty()) {
              MessageRow(text = "${filter.label}를 불러오는 중...", muted = true)
            }
          is QueryState.Success ->
            if (threads.isEmpty() && state.virtualThread == null) {
              MessageRow(
                text = if (filter == CommentFilter.Open) "열린 코멘트가 없어요." else "해결된 코멘트가 없어요.",
                muted = true,
              )
            }
        }

        threads.forEach { thread ->
          val active = state.activeThreadId == thread.id
          CommentThreadRow(
            thread = thread,
            filter = filter,
            active = active,
            location =
              if (filter == CommentFilter.Open) {
                threadLocationById[thread.id] ?: CommentThreadLocation.Missing
              } else {
                null
              },
            myId = myId,
            myRole = myRole,
            isOwner = isOwner,
            editingCommentId = state.editingCommentId,
            editingText = state.editingText,
            submittingDraft = commentSubmitInProgress,
            replyText = state.replyText(thread.id),
            onClick = { scope.launch { activateThread(thread) } },
            onStartEdit = { comment -> scope.launch { startEdit(comment) } },
            onEditingTextChange = state::updateEditingText,
            onCancelEdit = { scope.launch { cancelEdit() } },
            onSubmitEdit = { comment -> scope.launch { submitEdit(comment) } },
            onDeleteComment = { comment ->
              scope.launch {
                if (isRootComment(thread = thread, comment = comment)) {
                  deleteThread(thread)
                } else {
                  deleteComment(comment)
                }
              }
            },
            onReplyTextChange = { text ->
              state.updateReplyText(threadId = thread.id, text = text)
            },
            onSubmitReply = { content -> scope.launch { submitReply(thread.id, content) } },
            onInputFocusChanged = onInputFocusChanged,
            onResolve = { scope.launch { resolveThread(thread) } },
            onUnresolve = { scope.launch { unresolveThread(thread) } },
            onDeleteThread = { scope.launch { deleteThread(thread) } },
          )
        }

        state.virtualThread?.let { virtualThread ->
          key(virtualThread.selection) {
            VirtualCommentThreadRow(
              location = composeLocation,
              value = virtualThread.content,
              submitting = commentSubmitInProgress,
              onValueChange = state::updateVirtualContent,
              onFocusChange = onInputFocusChanged,
              onSubmit = { content -> scope.launch { submitVirtual(content) } },
            )
          }
        }

        Spacer(Modifier.height(CommentsListBottomContentPadding))
      }
    }
  }
}

@Composable
private fun CommentsSheetBar(
  selectedFilter: CommentFilter,
  count: Int,
  createEnabled: Boolean,
  onDismiss: () -> Unit,
  onFilterSelect: (CommentFilter) -> Unit,
  onCreate: () -> Unit,
  modifier: Modifier = Modifier,
) {
  Box(modifier = modifier.fillMaxWidth().height(44.dp)) {
    SheetBarButton(
      icon = Lucide.X,
      onClick = onDismiss,
      modifier = Modifier.align(Alignment.CenterStart),
    )

    Row(
      modifier = Modifier.align(Alignment.Center).padding(horizontal = 104.dp),
      horizontalArrangement = Arrangement.spacedBy(6.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(
        text = "코멘트",
        style = AppTheme.typography.title,
        color = AppTheme.colors.textDefault,
        overflow = TextOverflow.Ellipsis,
        maxLines = 1,
      )
      CountBadge(count = count)
    }

    Row(
      modifier = Modifier.align(Alignment.CenterEnd),
      horizontalArrangement = Arrangement.spacedBy(8.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      CommentsFilterPopover(selectedFilter = selectedFilter, onSelect = onFilterSelect)
      SheetBarButton(icon = Lucide.MessageSquarePlus, enabled = createEnabled, onClick = onCreate)
    }
  }
}

@Composable
private fun CommentsFilterPopover(
  selectedFilter: CommentFilter,
  onSelect: (CommentFilter) -> Unit,
) {
  PopoverMenu(anchor = { SheetBarButton(icon = Lucide.ListFilter, onClick = {}) }) {
    CommentFilter.entries.forEach { filter ->
      item(
        content = {
          Row(
            modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            Icon(
              icon = if (filter == CommentFilter.Resolved) Lucide.CircleCheck else Lucide.Circle,
              modifier = Modifier.size(18.dp),
              tint = AppTheme.colors.textMuted,
            )
            Text(
              text = filter.label,
              modifier = Modifier.weight(1f),
              style = AppTheme.typography.action,
            )
            Box(modifier = Modifier.width(28.dp), contentAlignment = Alignment.CenterEnd) {
              if (selectedFilter == filter) {
                Icon(
                  icon = Lucide.Check,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textDefault,
                )
              }
            }
          }
        },
        onClick = { onSelect(filter) },
      )
    }
  }
}

@Composable
private fun MessageRow(text: String, muted: Boolean) {
  Box(
    modifier = Modifier.fillMaxWidth().padding(vertical = 24.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = text,
      style = AppTheme.typography.body,
      color = if (muted) AppTheme.colors.textMuted else AppTheme.colors.textDefault,
    )
  }
}

private suspend fun confirmDiscardCommentInput(state: CommentThreadState, dialog: Dialog): Boolean {
  if (!state.hasUnsavedInput) {
    if (state.virtualThread != null) {
      state.clearVirtualThread()
    }
    return true
  }

  val result =
    dialog.confirm(
      title = "작성 중인 내용 삭제",
      message = "작성 중인 내용이 사라집니다. 계속할까요?",
      confirmText = "버리기",
      confirmIsDestructive = true,
    )
  if (result is DialogResult.Resolved) {
    state.discardUnsavedInput()
    return true
  }
  return false
}

private val CommentFilter.label: String
  get() =
    when (this) {
      CommentFilter.Open -> "열린 코멘트"
      CommentFilter.Resolved -> "해결된 코멘트"
    }
