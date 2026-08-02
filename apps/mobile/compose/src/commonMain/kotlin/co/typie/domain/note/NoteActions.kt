package co.typie.domain.note

import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateMapOf
import co.touchlab.kermit.Logger
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.type.NoteStatus
import co.typie.network.isRecoverableNetworkError
import co.typie.result.Result
import io.sentry.kotlin.multiplatform.Sentry

internal sealed interface NoteActionOutcome<out T> {
  data class Success<T>(val value: T) : NoteActionOutcome<T>

  data object Terminal : NoteActionOutcome<Nothing>

  data object Superseded : NoteActionOutcome<Nothing>

  data class Failure(val error: Throwable) : NoteActionOutcome<Nothing>
}

internal class NoteActionRequest
internal constructor(
  internal val siteId: String,
  internal val entityId: String?,
  internal val owner: Long,
)

@Stable
internal class NoteActions {
  private var siteId: String? = null
  private var entityId: String? = null
  private var editState: NoteEditState? = null
  private var onTerminal: (noteId: String) -> Unit = {}
  private var unregisterTerminalListener: (() -> Unit)? = null
  private var owner = 0L
  private var createRequest: Long? = null
  private val pendingDeletionRequests = mutableStateMapOf<String, Long>()
  private val statusChangeRequests = mutableStateMapOf<String, Long>()
  private var nextRequest = 0L

  fun activate(
    siteId: String?,
    entityId: String?,
    editState: NoteEditState,
    onTerminal: (noteId: String) -> Unit,
  ) {
    val changed = this.siteId != siteId || this.entityId != entityId || this.editState !== editState
    if (changed) {
      unregisterTerminalListener?.invoke()
      unregisterTerminalListener = null
      owner += 1
      createRequest = null
      pendingDeletionRequests.clear()
      statusChangeRequests.clear()
    }
    this.siteId = siteId
    this.entityId = entityId
    this.editState = editState
    this.onTerminal = onTerminal
    if (changed && siteId != null) {
      val operationOwner = owner
      unregisterTerminalListener =
        NoteSync.onTerminalDelete(siteId) { noteId ->
          if (owner == operationOwner) convergeTerminal(noteId)
        }
    }
  }

  fun dispose() {
    unregisterTerminalListener?.invoke()
    unregisterTerminalListener = null
    owner += 1
    siteId = null
    entityId = null
    editState = null
    onTerminal = {}
    createRequest = null
    pendingDeletionRequests.clear()
    statusChangeRequests.clear()
  }

  fun captureRequest(): NoteActionRequest? = siteId?.let {
    NoteActionRequest(siteId = it, entityId = entityId, owner = owner)
  }

  fun captureRequest(siteId: String, entityId: String?): NoteActionRequest? =
    captureRequest()?.takeIf { it.siteId == siteId && it.entityId == entityId }

  fun isCurrent(request: NoteActionRequest): Boolean = owns(request)

  fun isPendingDeletion(noteId: String): Boolean = noteId in pendingDeletionRequests

  fun isChangingStatus(noteId: String): Boolean = noteId in statusChangeRequests

  suspend fun open(
    request: NoteActionRequest,
    note: NoteCard_note,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    if (!owns(request)) return false
    val currentEditState = editState ?: return false
    val expandedNoteId = currentEditState.expandedNoteId
    val expandedNoteSiteId = currentEditState.expandedNoteSiteId
    if (
      expandedNoteId != null &&
        expandedNoteSiteId != null &&
        !currentEditState.isExpanded(siteId = note.site.id, noteId = note.id)
    ) {
      if (
        !currentEditState.flush(
          siteId = expandedNoteSiteId,
          noteId = expandedNoteId,
          saveContent = saveContent,
          saveColor = saveColor,
        )
      ) {
        return false
      }
    }
    if (!owns(request)) return false

    currentEditState.open(note = note)
    return true
  }

  suspend fun collapse(
    request: NoteActionRequest,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    if (!owns(request)) return false
    val currentEditState = editState ?: return false
    if (
      !currentEditState.collapse(
        siteId = request.siteId,
        saveContent = saveContent,
        saveColor = saveColor,
      )
    ) {
      return false
    }
    return owns(request)
  }

  suspend fun flush(
    request: NoteActionRequest,
    noteId: String,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ): Boolean {
    if (!owns(request)) return false
    val currentEditState = editState ?: return false
    if (
      !currentEditState.flush(
        siteId = request.siteId,
        noteId = noteId,
        saveContent = saveContent,
        saveColor = saveColor,
      )
    ) {
      return false
    }
    return owns(request)
  }

  suspend fun flushOnFocusLoss(
    request: NoteActionRequest,
    noteId: String,
    saveContent: suspend (noteId: String, content: String) -> NoteSaveOutcome,
    saveColor: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    if (!owns(request)) return
    val currentEditState = editState ?: return
    currentEditState.flushOnFocusLoss(
      siteId = request.siteId,
      noteId = noteId,
      saveContent = saveContent,
      saveColor = saveColor,
    )
  }

  fun updateContent(
    request: NoteActionRequest,
    noteId: String,
    value: String,
    save: suspend (noteId: String, content: String) -> NoteSaveOutcome,
  ) {
    if (!owns(request)) return
    editState?.updateContent(siteId = request.siteId, noteId = noteId, value = value, save = save)
  }

  fun updateColor(
    request: NoteActionRequest,
    noteId: String,
    value: String,
    save: suspend (noteId: String, color: String) -> NoteSaveOutcome,
  ) {
    if (!owns(request)) return
    editState?.updateColor(siteId = request.siteId, noteId = noteId, value = value, save = save)
  }

  fun listItems(notes: List<NoteCard_note>, state: NoteListState): List<NoteListItem> =
    notes.map { note ->
      NoteListItem(
        note = note,
        isDeleting = isPendingDeletion(note.id),
        isChangingStatus = isChangingStatus(note.id),
        isEntering = state.isEntering(note.id),
        isExiting = state.isExiting(note.id),
        isExitVisible = state.isExitVisible(note.id),
        isExitExpanded = state.isExitExpanded(note.id),
      )
    }

  suspend fun <T> create(
    request: NoteActionRequest,
    mutation: suspend (siteId: String) -> Result<T, Nothing>,
  ): NoteActionOutcome<T>? {
    if (!owns(request)) return NoteActionOutcome.Superseded
    if (createRequest != null) return null

    val requestId = ++nextRequest
    createRequest = requestId
    return try {
      val result = mutation(request.siteId)
      if (!owns(request)) {
        NoteActionOutcome.Superseded
      } else {
        resolveFailure(result)
      }
    } finally {
      if (owns(request) && createRequest == requestId) {
        createRequest = null
      }
    }
  }

  private suspend fun <T> execute(
    request: NoteActionRequest,
    noteId: String,
    mutation: suspend (siteId: String) -> Result<T, Nothing>,
  ): NoteActionOutcome<T> {
    if (!owns(request)) return NoteActionOutcome.Superseded
    val result = mutation(request.siteId)
    if (!owns(request)) return NoteActionOutcome.Superseded

    return resolve(noteId = noteId, siteId = request.siteId, result = result)
  }

  suspend fun delete(
    request: NoteActionRequest,
    noteId: String,
    mutation: suspend (siteId: String) -> Result<String, Nothing>,
  ): NoteActionOutcome<Nothing>? {
    if (!owns(request)) return NoteActionOutcome.Superseded
    if (noteId in pendingDeletionRequests) return null

    val requestId = ++nextRequest
    pendingDeletionRequests[noteId] = requestId
    editState?.cancelPendingSaves(siteId = request.siteId, noteId = noteId)

    return try {
      when (val outcome = execute(request = request, noteId = noteId, mutation = mutation)) {
        is NoteActionOutcome.Success -> NoteActionOutcome.Terminal
        NoteActionOutcome.Terminal -> NoteActionOutcome.Terminal
        NoteActionOutcome.Superseded -> NoteActionOutcome.Superseded
        is NoteActionOutcome.Failure -> outcome
      }
    } finally {
      if (owns(request) && pendingDeletionRequests[noteId] == requestId) {
        pendingDeletionRequests.remove(noteId)
      }
    }
  }

  suspend fun toggleStatus(
    request: NoteActionRequest,
    note: NoteCard_note,
    sourceState: NoteListState,
    destinationState: NoteListState,
    beforeMutation: suspend () -> Boolean,
    mutation: suspend (siteId: String, status: NoteStatus) -> Result<NoteCard_note, Nothing>,
  ): NoteActionOutcome<NoteCard_note>? {
    if (!owns(request)) return NoteActionOutcome.Superseded
    if (note.id in statusChangeRequests) return null

    val requestId = ++nextRequest
    statusChangeRequests[note.id] = requestId

    return try {
      if (!beforeMutation()) return null
      if (!owns(request)) return NoteActionOutcome.Superseded

      val nextStatus = note.status.toggled()
      val keepExpanded = editState?.isExpanded(siteId = request.siteId, noteId = note.id) == true
      val outcome =
        updateAndExit(
          request = request,
          note = note.copy(status = nextStatus),
          state = sourceState,
          keepExpanded = keepExpanded,
        ) { activeSiteId ->
          mutation(activeSiteId, nextStatus)
        }
      if (outcome is NoteActionOutcome.Success) {
        editState?.clearExpanded(siteId = request.siteId, noteId = note.id)
        destinationState.expectEntry(outcome.value)
      }
      outcome
    } finally {
      if (owns(request) && statusChangeRequests[note.id] == requestId) {
        statusChangeRequests.remove(note.id)
      }
    }
  }

  suspend fun updateAndExit(
    request: NoteActionRequest,
    note: NoteCard_note,
    state: NoteListState,
    keepExpanded: Boolean = false,
    mutation: suspend (siteId: String) -> Result<NoteCard_note, Nothing>,
  ): NoteActionOutcome<NoteCard_note> {
    if (!owns(request)) return NoteActionOutcome.Superseded

    val outcome = update(request = request, noteId = note.id, mutation = mutation)
    when (outcome) {
      is NoteActionOutcome.Success -> {
        state.markExiting(note = outcome.value, keepExpanded = keepExpanded)
        state.confirmMembershipRemoval(note.id)
      }
      NoteActionOutcome.Terminal -> {
        state.markExiting(note = note, keepExpanded = keepExpanded)
        state.confirmMembershipRemoval(note.id)
      }
      NoteActionOutcome.Superseded,
      is NoteActionOutcome.Failure -> Unit
    }
    return outcome
  }

  private fun <T> resolve(
    noteId: String,
    siteId: String,
    result: Result<T, Nothing>,
  ): NoteActionOutcome<T> {
    if (result.isNoteNotFound()) {
      NoteSync.markNotFound(siteId = siteId, noteId = noteId)
      return NoteActionOutcome.Terminal
    }
    if (NoteSync.isTerminallyDeleted(siteId, noteId)) {
      return NoteActionOutcome.Terminal
    }

    return resolveFailure(result)
  }

  private fun <T> resolveFailure(result: Result<T, Nothing>): NoteActionOutcome<T> {
    val outcome =
      when (result) {
        is Result.Ok -> NoteActionOutcome.Success(result.value)
        is Result.Exception -> NoteActionOutcome.Failure(result.exception)
        is Result.Err ->
          NoteActionOutcome.Failure(IllegalStateException("Unexpected note mutation error"))
      }
    if (outcome is NoteActionOutcome.Failure) {
      reportNoteMutationFailure(outcome.error)
    }
    return outcome
  }

  suspend fun save(
    request: NoteActionRequest,
    noteId: String,
    mutation: suspend (siteId: String) -> Result<NoteCard_note, Nothing>,
  ): NoteSaveOutcome {
    return when (val outcome = update(request = request, noteId = noteId, mutation = mutation)) {
      is NoteActionOutcome.Success,
      NoteActionOutcome.Terminal -> NoteSaveOutcome.Saved
      NoteActionOutcome.Superseded -> NoteSaveOutcome.Superseded
      is NoteActionOutcome.Failure -> NoteSaveOutcome.Failed
    }
  }

  suspend fun update(
    request: NoteActionRequest,
    noteId: String,
    mutation: suspend (siteId: String) -> Result<NoteCard_note, Nothing>,
  ): NoteActionOutcome<NoteCard_note> {
    val outcome = execute(request = request, noteId = noteId, mutation = mutation)
    if (outcome is NoteActionOutcome.Success) {
      editState?.commitServerSnapshot(outcome.value)
    }
    return outcome
  }

  private fun owns(request: NoteActionRequest): Boolean =
    owner == request.owner && siteId == request.siteId && entityId == request.entityId

  private fun convergeTerminal(noteId: String) {
    pendingDeletionRequests.remove(noteId)
    statusChangeRequests.remove(noteId)
    try {
      onTerminal(noteId)
    } catch (error: Throwable) {
      Logger.e(error) { "Failed to converge terminal note state" }
      runCatching { Sentry.captureException(error) }
    }
  }
}

internal fun Result<NoteCard_note, Nothing>.toNoteSaveOutcome(
  siteId: String,
  noteId: String,
): NoteSaveOutcome =
  when {
    isNoteNotFound() -> {
      NoteSync.markNotFound(siteId = siteId, noteId = noteId)
      NoteSaveOutcome.Saved
    }
    NoteSync.isTerminallyDeleted(siteId, noteId) -> NoteSaveOutcome.Saved
    this is Result.Ok -> NoteSaveOutcome.Saved
    this is Result.Exception -> {
      reportNoteMutationFailure(exception)
      NoteSaveOutcome.Failed
    }
    else -> {
      reportNoteMutationFailure(IllegalStateException("Unexpected note mutation error"))
      NoteSaveOutcome.Failed
    }
  }

private fun reportNoteMutationFailure(error: Throwable) {
  runCatching { Logger.e(error) { "Failed to mutate note" } }
  if (!error.isRecoverableNetworkError()) {
    runCatching { Sentry.captureException(error) }
  }
}
