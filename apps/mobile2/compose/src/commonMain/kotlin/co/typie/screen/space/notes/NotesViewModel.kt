package co.typie.screen.space.notes

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.domain.note.DEFAULT_NOTE_COLOR
import co.typie.domain.note.addNoteEntity as addNoteEntityMutation
import co.typie.domain.note.createNote as createNoteMutation
import co.typie.domain.note.deleteNote as deleteNoteMutation
import co.typie.domain.note.moveNote as moveNoteMutation
import co.typie.domain.note.removeNoteEntity as removeNoteEntityMutation
import co.typie.domain.note.updateNoteColor as updateNoteColorMutation
import co.typie.domain.note.updateNoteContent as updateNoteContentMutation
import co.typie.domain.note.updateNoteStatus as updateNoteStatusMutation
import co.typie.graphql.Apollo
import co.typie.graphql.NotesScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildNote
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.text
import co.typie.graphql.type.NoteStatus
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.storage.Preference
import kotlinx.coroutines.launch

internal class NotesViewModel : ViewModel() {
  private var hasEnteredScreen = false

  val siteId: String?
    get() = Preference.siteId

  val openQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(status = NoteStatus.OPEN),
      skip = { Preference.siteId == null },
    ) {
      NotesScreen_Query(siteId = Preference.siteId!!, status = NoteStatus.OPEN)
    }

  val resolvedQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(status = NoteStatus.RESOLVED),
      skip = { Preference.siteId == null },
    ) {
      NotesScreen_Query(siteId = Preference.siteId!!, status = NoteStatus.RESOLVED)
    }

  fun query(status: NoteStatus) =
    when (status) {
      NoteStatus.RESOLVED -> resolvedQuery
      else -> openQuery
    }

  fun notes(status: NoteStatus): List<NoteCard_note> = query(status).data.notes()

  fun settledNotes(status: NoteStatus): List<NoteCard_note> =
    (query(status).state as? QueryState.Success)?.data?.notes().orEmpty()

  fun refetch() {
    refetch(NoteStatus.OPEN)
    refetch(NoteStatus.RESOLVED)
  }

  fun refetch(status: NoteStatus) {
    if (siteId == null) {
      return
    }

    query(status).refetch()
  }

  fun onScreenEntered() {
    if (hasEnteredScreen) {
      refetch()
      return
    }

    hasEnteredScreen = true
  }

  suspend fun createNote(color: String = DEFAULT_NOTE_COLOR): Result<NoteCard_note, Nothing> =
    createNoteMutation(color = color)

  suspend fun updateNoteContent(noteId: String, content: String): Result<NoteCard_note, Nothing> =
    updateNoteContentMutation(noteId = noteId, content = content)

  suspend fun updateNoteColor(noteId: String, color: String): Result<NoteCard_note, Nothing> =
    updateNoteColorMutation(noteId = noteId, color = color)

  suspend fun updateNoteStatus(noteId: String, status: NoteStatus): Result<NoteCard_note, Nothing> =
    updateNoteStatusMutation(noteId = noteId, status = status)

  suspend fun deleteNote(noteId: String): Result<Unit, Nothing> =
    deleteNoteMutation(noteId = noteId)

  suspend fun moveNote(
    noteId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> =
    moveNoteMutation(noteId = noteId, lowerOrder = lowerOrder, upperOrder = upperOrder)

  suspend fun addNoteEntity(noteId: String, entityId: String): Result<NoteCard_note, Nothing> =
    addNoteEntityMutation(noteId = noteId, entityId = entityId)

  suspend fun removeNoteEntity(noteId: String, entityId: String): Result<NoteCard_note, Nothing> =
    removeNoteEntityMutation(noteId = noteId, entityId = entityId)

  fun savePendingNoteContent(noteId: String, content: String) {
    viewModelScope.launch { updateNoteContent(noteId = noteId, content = content) }
  }

  fun savePendingNoteColor(noteId: String, color: String) {
    viewModelScope.launch { updateNoteColor(noteId = noteId, color = color) }
  }
}

private fun placeholderData(status: NoteStatus) =
  NotesScreen_Query.Data(PlaceholderResolver) {
    notes =
      List(3) { index ->
        buildNote {
          id = "placeholder-note-$index"
          content = text(14..26, lines = if (index == 0) 1 else 2)
          order = index.toString()
          color = DEFAULT_NOTE_COLOR
          this.status = status
          entities = emptyList()
        }
      }
  }

internal fun NotesScreen_Query.Data.notes(): List<NoteCard_note> =
  notes.map { it.noteCard_note }.sortedBy { it.order }
