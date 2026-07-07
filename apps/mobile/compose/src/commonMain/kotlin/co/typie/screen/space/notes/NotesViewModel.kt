package co.typie.screen.space.notes

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
  val noteEditState = NoteEditState(scope = viewModelScope)

  val siteId: String?
    get() = Preference.siteId

  var filterStatus by mutableStateOf(NoteStatus.OPEN)
    private set

  private val sceneCache = NotesSceneCache()

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      skip = { Preference.siteId == null },
      resetOnChange = false,
    ) {
      NotesScreen_Query(siteId = Preference.siteId!!, status = filterStatus)
    }

  init {
    viewModelScope.launch {
      snapshotFlow { query.state }
        .collect { state ->
          if (state is QueryState.Success) {
            val notes = state.data.notes()
            val key = queryStateKey() ?: return@collect
            sceneCache.commitSuccess(key, notes)

            val activeNoteId = noteEditState.expandedNoteId ?: return@collect
            notes.firstOrNull { it.id == activeNoteId }?.let(noteEditState::commitServerSnapshot)
          }
        }
    }
  }

  fun listState(status: NoteStatus): NoteListState =
    sceneKey(status)?.let(sceneCache::listState) ?: sceneCache.fallbackListState(status)

  fun updateFilterStatus(status: NoteStatus) {
    if (status == NoteStatus.UNKNOWN__ || filterStatus == status) {
      return
    }

    filterStatus = status
  }

  fun notes(status: NoteStatus): List<NoteCard_note> =
    sceneCache.notes(
      key = sceneKey(status),
      queryKey = queryStateKey(),
      queryState = queryNotesState(),
      placeholderNotes = placeholderNotes(status),
    )

  fun queryState(status: NoteStatus): QueryState<*> =
    sceneCache.queryState(
      key = sceneKey(status),
      activeKey = sceneKey(filterStatus),
      queryKey = queryStateKey(),
      queryState = queryNotesState(),
    )

  fun refetch() {
    if (siteId == null) {
      return
    }

    query.refetch()
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

  private fun sceneKey(status: NoteStatus): NotesSceneKey? = siteId?.let {
    NotesSceneKey(siteId = it, status = status)
  }

  private fun queryStateKey(): NotesSceneKey? =
    (query.stateQuery as? NotesScreen_Query)?.let { query ->
      NotesSceneKey(siteId = query.siteId, status = query.status)
    }

  private fun queryNotesState(): QueryState<List<NoteCard_note>> =
    when (val state = query.state) {
      is QueryState.Success -> QueryState.Success(state.data.notes())
      is QueryState.Error -> state
      QueryState.Loading -> QueryState.Loading
    }
}

internal data class NotesSceneKey(val siteId: String, val status: NoteStatus)

internal class NotesSceneCache {
  private val settledNotesByKey = mutableStateMapOf<NotesSceneKey, List<NoteCard_note>>()
  private val listStatesByKey = mutableMapOf<NotesSceneKey, NoteListState>()
  private val fallbackListStates = mutableMapOf<NoteStatus, NoteListState>()

  fun commitSuccess(key: NotesSceneKey, notes: List<NoteCard_note>) {
    settledNotesByKey[key] = notes
    listState(key).sync(notes)
  }

  fun listState(key: NotesSceneKey): NoteListState =
    listStatesByKey.getOrPut(key) { NoteListState(key.status.normalizedListStatus()) }

  fun fallbackListState(status: NoteStatus): NoteListState =
    fallbackListStates.getOrPut(status.normalizedListStatus()) {
      NoteListState(status.normalizedListStatus())
    }

  fun notes(
    key: NotesSceneKey?,
    queryKey: NotesSceneKey?,
    queryState: QueryState<List<NoteCard_note>>,
    placeholderNotes: List<NoteCard_note>,
  ): List<NoteCard_note> =
    when {
      key != null && key == queryKey && queryState is QueryState.Success -> queryState.data
      key != null && key in settledNotesByKey -> settledNotesByKey.getValue(key)
      else -> placeholderNotes
    }

  fun queryState(
    key: NotesSceneKey?,
    activeKey: NotesSceneKey?,
    queryKey: NotesSceneKey?,
    queryState: QueryState<List<NoteCard_note>>,
  ): QueryState<*> =
    when {
      key != null &&
        key == activeKey &&
        key == queryKey &&
        queryState is QueryState.Error &&
        key !in settledNotesByKey -> queryState
      key != null && key == queryKey && queryState is QueryState.Success -> QueryState.Success(Unit)
      key != null && key in settledNotesByKey -> QueryState.Success(Unit)
      else -> QueryState.Loading
    }
}

private fun NoteStatus.normalizedListStatus(): NoteStatus =
  when (this) {
    NoteStatus.RESOLVED -> NoteStatus.RESOLVED
    else -> NoteStatus.OPEN
  }

private val openPlaceholderNotes = placeholderData(status = NoteStatus.OPEN).notes()
private val resolvedPlaceholderNotes = placeholderData(status = NoteStatus.RESOLVED).notes()

private fun placeholderNotes(status: NoteStatus): List<NoteCard_note> =
  when (status) {
    NoteStatus.RESOLVED -> resolvedPlaceholderNotes
    else -> openPlaceholderNotes
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
