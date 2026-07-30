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
      NotesScreen_Query(siteId = Preference.siteId!!)
    }

  init {
    viewModelScope.launch {
      snapshotFlow { query.state }
        .collect { state ->
          if (state is QueryState.Success) {
            val responseSiteId = queryStateSiteId() ?: return@collect
            if (responseSiteId != siteId) return@collect
            val notes =
              state.data.notes().filterNot { NoteSync.isTerminallyDeleted(responseSiteId, it.id) }
            sceneCache.commitSuccess(siteId = responseSiteId, notes = notes)

            val activeNoteId = noteEditState.expandedNoteId ?: return@collect
            notes.firstOrNull { it.id == activeNoteId }?.let(noteEditState::commitServerSnapshot)
          }
        }
    }

    viewModelScope.launch {
      snapshotFlow { siteId }.collect { currentSiteId -> sceneCache.activateSite(currentSiteId) }
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
      querySiteId = queryStateSiteId(),
      queryState = queryNotesState(),
      placeholderNotes = placeholderNotes(status),
    )

  fun queryState(status: NoteStatus): QueryState<*> =
    sceneCache.queryState(
      key = sceneKey(status),
      querySiteId = queryStateSiteId(),
      queryState = queryNotesState(),
    )

  fun refetch() {
    if (siteId == null) {
      return
    }

    query.refetch()
  }

  suspend fun createNote(
    siteId: String,
    color: String = DEFAULT_NOTE_COLOR,
  ): Result<NoteCard_note, Nothing> = createNoteMutation(siteId = siteId, color = color)

  suspend fun updateNoteContent(
    siteId: String,
    noteId: String,
    content: String,
  ): Result<NoteCard_note, Nothing> =
    updateNoteContentMutation(siteId = siteId, noteId = noteId, content = content)

  suspend fun updateNoteColor(
    siteId: String,
    noteId: String,
    color: String,
  ): Result<NoteCard_note, Nothing> =
    updateNoteColorMutation(siteId = siteId, noteId = noteId, color = color)

  suspend fun updateNoteStatus(
    siteId: String,
    noteId: String,
    status: NoteStatus,
  ): Result<NoteCard_note, Nothing> =
    updateNoteStatusMutation(siteId = siteId, noteId = noteId, status = status)

  suspend fun deleteNote(siteId: String, noteId: String): Result<String, Nothing> =
    deleteNoteMutation(siteId = siteId, noteId = noteId)

  suspend fun moveNote(
    note: NoteCard_note,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<String, Nothing> =
    moveNoteMutation(note = note, lowerOrder = lowerOrder, upperOrder = upperOrder)

  suspend fun addNoteEntity(
    siteId: String,
    noteId: String,
    entityId: String,
  ): Result<NoteCard_note, Nothing> =
    addNoteEntityMutation(siteId = siteId, noteId = noteId, entityId = entityId)

  suspend fun removeNoteEntity(
    siteId: String,
    noteId: String,
    entityId: String,
  ): Result<NoteCard_note, Nothing> =
    removeNoteEntityMutation(siteId = siteId, noteId = noteId, entityId = entityId)

  suspend fun savePendingNoteContent(
    siteId: String,
    noteId: String,
    content: String,
  ): NoteSaveOutcome {
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      return NoteSaveOutcome.SubscriptionGated
    }
    return updateNoteContent(siteId = siteId, noteId = noteId, content = content)
      .toNoteSaveOutcome(siteId = siteId, noteId = noteId)
  }

  suspend fun savePendingNoteColor(siteId: String, noteId: String, color: String): NoteSaveOutcome {
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      return NoteSaveOutcome.SubscriptionGated
    }
    return updateNoteColor(siteId = siteId, noteId = noteId, color = color)
      .toNoteSaveOutcome(siteId = siteId, noteId = noteId)
  }

  fun convergeDeletedNote(noteId: String) {
    siteId?.let { noteEditState.remove(siteId = it, noteId = noteId) }
    sceneCache.convergeDeletedNote(
      noteId = noteId,
      querySiteId = queryStateSiteId(),
      queryNotes = (queryNotesState() as? QueryState.Success)?.data,
    )
  }

  private fun sceneKey(status: NoteStatus): NotesSceneKey? = siteId?.let {
    NotesSceneKey(siteId = it, status = status)
  }

  private fun queryStateSiteId(): String? = (query.stateQuery as? NotesScreen_Query)?.siteId

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
  private var activeSiteId: String? = null
  private var hasActiveSite = false

  fun activateSite(siteId: String?) {
    if (hasActiveSite && activeSiteId == siteId) return

    activeSiteId = siteId
    hasActiveSite = true
    settledNotesByKey.clear()
    listStatesByKey.clear()
    fallbackListStates.clear()
  }

  fun commitSuccess(siteId: String, notes: List<NoteCard_note>) {
    if (!hasActiveSite) activateSite(siteId)
    if (activeSiteId != siteId) return

    val currentNotes = notes.withoutTerminalNotes(siteId)
    listOf(NoteStatus.OPEN, NoteStatus.RESOLVED).forEach { status ->
      val key = NotesSceneKey(siteId = siteId, status = status)
      settledNotesByKey[key] = currentNotes.filter { it.status == status }
      listState(key).sync(currentNotes)
    }
  }

  fun listState(key: NotesSceneKey): NoteListState {
    activateSite(key.siteId)
    return listStatesByKey.getOrPut(key) { NoteListState(key.status.normalizedListStatus()) }
  }

  fun fallbackListState(status: NoteStatus): NoteListState =
    fallbackListStates.getOrPut(status.normalizedListStatus()) {
      NoteListState(status.normalizedListStatus())
    }

  fun convergeDeletedNote(noteId: String, querySiteId: String?, queryNotes: List<NoteCard_note>?) {
    listStatesByKey.forEach { (key, state) ->
      val note =
        settledNotesByKey[key]?.firstOrNull { it.id == noteId }
          ?: if (key.siteId == querySiteId) {
            queryNotes?.firstOrNull { it.id == noteId && it.status == key.status }
          } else {
            null
          }
      state.markDeleted(noteId = noteId, fallbackNote = note)
    }
    fallbackListStates.values.forEach { it.markDeleted(noteId) }
  }

  fun notes(
    key: NotesSceneKey?,
    querySiteId: String?,
    queryState: QueryState<List<NoteCard_note>>,
    placeholderNotes: List<NoteCard_note>,
  ): List<NoteCard_note> =
    when {
      key != null && hasActiveSite && key.siteId != activeSiteId -> placeholderNotes
      key != null && key.siteId == querySiteId && queryState is QueryState.Success ->
        queryState.data.withoutTerminalNotes(key.siteId).filter { it.status == key.status }
      key != null && key in settledNotesByKey -> settledNotesByKey.getValue(key)
      else -> placeholderNotes
    }

  fun queryState(
    key: NotesSceneKey?,
    querySiteId: String?,
    queryState: QueryState<List<NoteCard_note>>,
  ): QueryState<*> {
    if (key == null || (hasActiveSite && key.siteId != activeSiteId)) {
      return QueryState.Loading
    }
    if (key in settledNotesByKey) {
      return QueryState.Success(Unit)
    }
    if (key.siteId != querySiteId) {
      return QueryState.Loading
    }

    return when (queryState) {
      is QueryState.Success -> QueryState.Success(Unit)
      is QueryState.Error -> queryState
      QueryState.Loading -> QueryState.Loading
    }
  }

  private fun List<NoteCard_note>.withoutTerminalNotes(siteId: String): List<NoteCard_note> =
    filterNot {
      NoteSync.isTerminallyDeleted(siteId, it.id)
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
