package co.typie.screen.editor.editor.subpane.relatednotes

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.note.DEFAULT_NOTE_COLOR
import co.typie.domain.note.NoteEditState
import co.typie.domain.note.NoteListState
import co.typie.domain.note.NoteSaveOutcome
import co.typie.domain.note.NoteSync
import co.typie.domain.note.addNoteEntity as addNoteEntityMutation
import co.typie.domain.note.createNote as createNoteMutation
import co.typie.domain.note.deleteNote as deleteNoteMutation
import co.typie.domain.note.moveNote as moveNoteMutation
import co.typie.domain.note.removeNoteEntity as removeNoteEntityMutation
import co.typie.domain.note.toNoteSaveOutcome
import co.typie.domain.note.updateNoteColor as updateNoteColorMutation
import co.typie.domain.note.updateNoteContent as updateNoteContentMutation
import co.typie.domain.note.updateNoteStatus as updateNoteStatusMutation
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.SubscriptionService
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.RelatedNotesSheet_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildNote
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.text
import co.typie.graphql.type.NoteStatus
import co.typie.graphql.watchQuery
import co.typie.result.Result
import kotlinx.coroutines.launch

internal class RelatedNotesViewModel(private val entityId: String, val siteId: String) :
  ViewModel() {
  val noteEditState = NoteEditState(scope = viewModelScope)

  var filterStatus by mutableStateOf(NoteStatus.OPEN)
    private set

  private val settledNotesByStatus = mutableStateMapOf<NoteStatus, List<NoteCard_note>>()
  private val openListState = NoteListState(NoteStatus.OPEN)
  private val resolvedListState = NoteListState(NoteStatus.RESOLVED)

  val query =
    Apollo.watchQuery(scope = viewModelScope, resetOnChange = false) {
      RelatedNotesSheet_Query(entityId = entityId)
    }

  init {
    viewModelScope.launch {
      snapshotFlow { query.state }
        .collect { state ->
          if (state is QueryState.Success) {
            val notes = state.data.notes().filterNot { NoteSync.isTerminallyDeleted(siteId, it.id) }
            listOf(NoteStatus.OPEN, NoteStatus.RESOLVED).forEach { status ->
              settledNotesByStatus[status] = notes.filter { it.status == status }
              listState(status).sync(notes)
            }

            val activeNoteId = noteEditState.expandedNoteId ?: return@collect
            notes.firstOrNull { it.id == activeNoteId }?.let(noteEditState::commitServerSnapshot)
          }
        }
    }

    viewModelScope.launch {
      NoteSync.updates.collect { update ->
        if (update.siteId != siteId) {
          return@collect
        }

        refetch()
      }
    }
  }

  fun listState(status: NoteStatus): NoteListState =
    when (status) {
      NoteStatus.RESOLVED -> resolvedListState
      else -> openListState
    }

  fun updateFilterStatus(status: NoteStatus) {
    if (status == NoteStatus.UNKNOWN__ || filterStatus == status) {
      return
    }

    filterStatus = status
  }

  fun notes(status: NoteStatus): List<NoteCard_note> {
    return when {
      query.state is QueryState.Success ->
        (query.state as QueryState.Success).data.notes().filter {
          it.status == status && !NoteSync.isTerminallyDeleted(siteId, it.id)
        }
      status in settledNotesByStatus -> settledNotesByStatus.getValue(status)
      else -> placeholderNotes(status)
    }
  }

  fun queryState(status: NoteStatus): QueryState<*> {
    return when {
      status in settledNotesByStatus -> QueryState.Success(Unit)
      query.state is QueryState.Success -> QueryState.Success(Unit)
      query.state is QueryState.Error -> query.state as QueryState.Error
      else -> QueryState.Loading
    }
  }

  fun refetch() {
    query.refetch()
  }

  suspend fun createNote(color: String = DEFAULT_NOTE_COLOR): Result<NoteCard_note, Nothing> =
    createNoteMutation(siteId = siteId, color = color, entityId = entityId)

  suspend fun updateNoteContent(noteId: String, content: String): Result<NoteCard_note, Nothing> =
    updateNoteContentMutation(siteId = siteId, noteId = noteId, content = content)

  suspend fun updateNoteColor(noteId: String, color: String): Result<NoteCard_note, Nothing> =
    updateNoteColorMutation(siteId = siteId, noteId = noteId, color = color)

  suspend fun updateNoteStatus(noteId: String, status: NoteStatus): Result<NoteCard_note, Nothing> =
    updateNoteStatusMutation(siteId = siteId, noteId = noteId, status = status)

  suspend fun deleteNote(noteId: String): Result<String, Nothing> =
    deleteNoteMutation(siteId = siteId, noteId = noteId)

  suspend fun moveNote(
    note: NoteCard_note,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<String, Nothing> =
    moveNoteMutation(note = note, lowerOrder = lowerOrder, upperOrder = upperOrder)

  suspend fun addNoteEntity(noteId: String, entityId: String): Result<NoteCard_note, Nothing> =
    addNoteEntityMutation(siteId = siteId, noteId = noteId, entityId = entityId)

  suspend fun removeNoteEntity(noteId: String, entityId: String): Result<NoteCard_note, Nothing> =
    removeNoteEntityMutation(siteId = siteId, noteId = noteId, entityId = entityId)

  suspend fun savePendingNoteContent(
    siteId: String,
    noteId: String,
    content: String,
  ): NoteSaveOutcome {
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      return NoteSaveOutcome.SubscriptionGated
    }
    return updateNoteContentMutation(siteId = siteId, noteId = noteId, content = content)
      .toNoteSaveOutcome(siteId = siteId, noteId = noteId)
  }

  suspend fun savePendingNoteColor(siteId: String, noteId: String, color: String): NoteSaveOutcome {
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      return NoteSaveOutcome.SubscriptionGated
    }
    return updateNoteColorMutation(siteId = siteId, noteId = noteId, color = color)
      .toNoteSaveOutcome(siteId = siteId, noteId = noteId)
  }

  fun convergeDeletedNote(noteId: String) {
    noteEditState.remove(siteId = siteId, noteId = noteId)
    val queryState = query.state
    val liveNotes =
      if (queryState is QueryState.Success) {
        queryState.data.notes().firstOrNull { it.id == noteId }
      } else {
        null
      }
    listOf(NoteStatus.OPEN, NoteStatus.RESOLVED).forEach { status ->
      val note =
        settledNotesByStatus[status]?.firstOrNull { it.id == noteId }
          ?: liveNotes?.takeIf { it.status == status }
      listState(status).markDeleted(noteId = noteId, fallbackNote = note)
    }
  }
}

private val openPlaceholderNotes = placeholderData(status = NoteStatus.OPEN).notes()
private val resolvedPlaceholderNotes = placeholderData(status = NoteStatus.RESOLVED).notes()

private fun placeholderNotes(status: NoteStatus): List<NoteCard_note> =
  when (status) {
    NoteStatus.RESOLVED -> resolvedPlaceholderNotes
    else -> openPlaceholderNotes
  }

private fun placeholderData(status: NoteStatus) =
  RelatedNotesSheet_Query.Data(PlaceholderResolver) {
    notes =
      List(3) { index ->
        buildNote {
          id = "placeholder-related-note-$index"
          content = text(14..26, lines = if (index == 0) 1 else 2)
          order = index.toString()
          color = DEFAULT_NOTE_COLOR
          this.status = status
          entities = emptyList()
        }
      }
  }

internal fun RelatedNotesSheet_Query.Data.notes(): List<NoteCard_note> =
  notes.map { it.noteCard_note }.sortedBy { it.order }
