package co.typie.screen.editor.editor.subpane.comments

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.toPageRectsTarget
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import kotlinx.coroutines.launch

internal class EditorCommentsSession(
  val model: CommentsViewModel?,
  val pendingRequest: PendingCommentSheetRequest?,
  val consumePendingRequest: (PendingCommentSheetRequest) -> Unit,
  val toolbarEnabled: Boolean,
  val topBarCreateEnabled: Boolean,
  val threadLocationById: Map<String, CommentThreadLocation>,
  val composeLocation: CommentThreadLocation?,
  val virtualThreadGuardVisible: Boolean,
  val freezeCurrentSelection: suspend () -> StableSelection?,
  val onInputFocusChanged: (Boolean) -> Unit,
  val requestFromTextToolbar: () -> Unit,
  val openFromToolPanel: () -> Unit,
  private val onDiscardVirtualThreadRequested: () -> Unit,
) {
  fun requestDiscardVirtualThread() {
    onDiscardVirtualThreadRequested()
  }
}

internal data class PendingCommentSheetRequest(val id: Int, val intent: CommentSheetRequest)

internal sealed interface CommentSheetRequest {
  data class Create(val selection: StableSelection?) : CommentSheetRequest

  data class ActivateThread(val threadId: String?) : CommentSheetRequest

  data object DiscardVirtualThread : CommentSheetRequest
}

@Stable
private class CommentSheetRequestQueue {
  private var nextRequestId = 0
  private val pendingRequests = mutableStateListOf<PendingCommentSheetRequest>()

  val pendingRequest: PendingCommentSheetRequest?
    get() = pendingRequests.firstOrNull()

  fun requestCreate(selection: StableSelection?) {
    enqueue(CommentSheetRequest.Create(selection = selection))
  }

  fun requestActiveThread(threadId: String?) {
    enqueue(CommentSheetRequest.ActivateThread(threadId = threadId))
  }

  fun requestDiscardVirtualThread() {
    enqueue(CommentSheetRequest.DiscardVirtualThread)
  }

  fun consume(request: PendingCommentSheetRequest) {
    pendingRequests.remove(request)
  }

  private fun enqueue(intent: CommentSheetRequest) {
    pendingRequests += PendingCommentSheetRequest(id = nextRequestId++, intent = intent)
  }
}

@Composable
internal fun rememberEditorCommentsSession(
  entityId: String,
  documentId: String?,
  documentLocked: Boolean,
  editor: Editor?,
  editorState: EditorState,
  sheetActive: Boolean,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  hideContextMenu: () -> Unit,
  openSheet: () -> Unit,
): EditorCommentsSession {
  val requestQueue = remember(entityId) { CommentSheetRequestQueue() }
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  var inputFocused by remember(entityId) { mutableStateOf(false) }
  val model = documentId?.let { id ->
    viewModel(key = "editor-comments:$id") {
      CommentsViewModel(entityId = entityId, documentId = id)
    }
  }

  LaunchedEffect(editor) { editor?.installCommentDecorations() }

  val selection = editorState.selection
  val selectionCollapsed = selection == null || selection.anchor == selection.head
  val collapsedCommentRanges =
    remember(editorState.trackedRangesContainingSelectionHead, selectionCollapsed, documentLocked) {
      if (selection != null && selectionCollapsed && !documentLocked) {
        editorState.trackedRangesContainingSelectionHead.commentRangeEndpoints()
      } else {
        emptyList()
      }
    }
  val collapsedCommentRangeIds = collapsedCommentRanges.mapTo(mutableSetOf()) { it.id }
  val collapsedSelectionHead = selection?.takeIf { selectionCollapsed }?.head
  val topBarCreateEnabled =
    model != null && !documentLocked && selection != null && !selectionCollapsed
  val toolbarEnabled =
    model != null &&
      !documentLocked &&
      selection != null &&
      (!selectionCollapsed || collapsedCommentRanges.isNotEmpty())

  val trackedCommentRanges =
    remember(editorState.trackedRanges, sheetActive) {
      if (sheetActive) {
        editorState.trackedRanges.filter {
          it.group == COMMENT_RANGE_GROUP ||
            it.group == ACTIVE_COMMENT_RANGE_GROUP ||
            it.group == COMMENT_COMPOSE_RANGE_GROUP
        }
      } else {
        emptyList()
      }
    }
  val trackedCommentRangeById =
    remember(trackedCommentRanges) { trackedCommentRanges.associateBy { it.id } }
  val visibleFilter = model?.threadState?.filter ?: CommentFilter.Open
  val activeThreadId = model?.threadState?.activeThreadId
  var lastRequestedActiveThreadId by remember(editor) { mutableStateOf<String?>(null) }
  val activeThreadScrollTarget =
    remember(activeThreadId, trackedCommentRanges) {
      trackedCommentRanges.commentThreadScrollTarget(activeThreadId)
    }
  val visibleThreads =
    if (sheetActive && visibleFilter == CommentFilter.Open) {
      model?.threads(visibleFilter).orEmpty()
    } else {
      emptyList()
    }
  val threadLocationById =
    remember(
      editor,
      editorState.version,
      sheetActive,
      visibleFilter,
      visibleThreads,
      trackedCommentRangeById,
    ) {
      if (!sheetActive || visibleFilter != CommentFilter.Open) {
        emptyMap()
      } else {
        visibleThreads.associate { thread ->
          thread.id to commentThreadLocation(trackedCommentRangeById[thread.id]?.text)
        }
      }
    }
  val composeLocation =
    trackedCommentRangeById[COMMENT_COMPOSE_RANGE_ID]?.let { range ->
      commentThreadLocation(range.text)
    }
  val composeSelection = model?.threadState?.virtualThread?.selection
  val openSelectionsById = model?.openSelectionsById.orEmpty()
  val openSelectionDecodeFailureIds = model?.openSelectionDecodeFailureIds.orEmpty()

  LaunchedEffect(openSelectionDecodeFailureIds) {
    if (openSelectionDecodeFailureIds.isNotEmpty()) {
      toast.show(ToastType.Error, "코멘트 위치 정보를 읽을 수 없어요.")
    }
  }

  LaunchedEffect(editor, openSelectionsById, activeThreadId) {
    val activeEditor = editor ?: return@LaunchedEffect
    activeEditor.syncCommentRanges(
      selectionsById = openSelectionsById,
      activeId = activeThreadId,
      currentRanges = editorState.trackedRanges,
    )
  }
  LaunchedEffect(editor, composeSelection) { editor?.setCommentComposeRange(composeSelection) }
  LaunchedEffect(editor, activeThreadId, activeThreadScrollTarget) {
    val threadId = activeThreadId
    if (threadId == null) {
      lastRequestedActiveThreadId = null
      return@LaunchedEffect
    }
    val target = activeThreadScrollTarget ?: return@LaunchedEffect
    if (lastRequestedActiveThreadId == threadId) {
      return@LaunchedEffect
    }
    val activeEditor = editor ?: return@LaunchedEffect
    bringIntoViewRequests.requestForVersion(
      target = target,
      version = activeEditor.state.version,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )
    lastRequestedActiveThreadId = threadId
  }
  LaunchedEffect(sheetActive, collapsedSelectionHead, collapsedCommentRangeIds) {
    if (
      !sheetActive ||
        selection == null ||
        !selectionCollapsed ||
        inputFocused ||
        model?.threadState?.hasUnsavedInput == true
    ) {
      return@LaunchedEffect
    }

    val threadId = collapsedCommentRanges.firstOrNull()?.id ?: return@LaunchedEffect
    if (threadId != activeThreadId) {
      requestQueue.requestActiveThread(threadId)
    }
  }

  return EditorCommentsSession(
    model = model,
    pendingRequest = requestQueue.pendingRequest,
    consumePendingRequest = requestQueue::consume,
    toolbarEnabled = toolbarEnabled,
    topBarCreateEnabled = topBarCreateEnabled,
    threadLocationById = threadLocationById,
    composeLocation = composeLocation,
    virtualThreadGuardVisible = sheetActive && model?.threadState?.hasDirtyVirtualThread == true,
    freezeCurrentSelection = {
      editorState.selection?.let { currentSelection -> editor?.freezeSelection(currentSelection) }
    },
    onInputFocusChanged = { focused -> inputFocused = focused },
    requestFromTextToolbar = {
      hideContextMenu()
      val activeEditor = editor
      val currentSelection = editorState.selection
      if (currentSelection != null && activeEditor != null) {
        if (currentSelection.anchor != currentSelection.head) {
          scope.launch {
            val frozenSelection = activeEditor.freezeSelection(currentSelection)
            if (frozenSelection == null) {
              toast.show(ToastType.Error, "선택 영역에 코멘트를 달 수 없어요.")
            } else {
              openSheet()
              requestQueue.requestCreate(frozenSelection)
            }
          }
        } else {
          openSheet()
          collapsedCommentRanges.firstOrNull()?.let { range ->
            requestQueue.requestActiveThread(range.id)
          }
        }
      }
    },
    openFromToolPanel = {
      hideContextMenu()
      if (model != null) {
        openSheet()
        if (selection != null && selectionCollapsed) {
          collapsedCommentRanges.firstOrNull()?.let { range ->
            requestQueue.requestActiveThread(range.id)
          }
        }
      }
    },
    onDiscardVirtualThreadRequested = requestQueue::requestDiscardVirtualThread,
  )
}

private fun List<TrackedRange>.commentThreadScrollTarget(
  threadId: String?
): EditorBringIntoViewTarget? {
  if (threadId == null) {
    return null
  }
  return commentRanges().firstOrNull { it.id == threadId }?.rects?.toPageRectsTarget()
}
