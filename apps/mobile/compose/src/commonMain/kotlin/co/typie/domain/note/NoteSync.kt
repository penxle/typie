package co.typie.domain.note

import co.typie.graphql.TypieError
import co.typie.graphql.type.NoteUpdateKind
import co.typie.result.Result
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.update

internal data class NoteUpdate(val kind: NoteUpdateKind, val noteId: String, val siteId: String)

internal object NoteSync {
  @OptIn(ExperimentalUuidApi::class) val clientId: String = Uuid.random().toHexString()

  private val mutableUpdates =
    MutableSharedFlow<NoteUpdate>(
      extraBufferCapacity = 64,
      onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
  private val terminalNoteIdsBySite = MutableStateFlow<Map<String, Set<String>>>(emptyMap())
  private val terminalListenersBySite = mutableMapOf<String, MutableSet<(String) -> Unit>>()
  val updates = mutableUpdates.asSharedFlow()

  fun publish(update: NoteUpdate) {
    when (update.kind) {
      NoteUpdateKind.CREATED ->
        terminalNoteIdsBySite.update { noteIdsBySite ->
          val noteIds = noteIdsBySite[update.siteId] ?: return@update noteIdsBySite
          val nextNoteIds = noteIds - update.noteId
          if (nextNoteIds.isEmpty()) noteIdsBySite - update.siteId
          else noteIdsBySite + (update.siteId to nextNoteIds)
        }
      NoteUpdateKind.DELETED ->
        markTerminallyDeleted(siteId = update.siteId, noteId = update.noteId)
      NoteUpdateKind.UPDATED,
      NoteUpdateKind.UNKNOWN__ -> Unit
    }
    mutableUpdates.tryEmit(update)
  }

  fun markNotFound(siteId: String, noteId: String) {
    if (markTerminallyDeleted(siteId = siteId, noteId = noteId)) {
      mutableUpdates.tryEmit(
        NoteUpdate(kind = NoteUpdateKind.DELETED, noteId = noteId, siteId = siteId)
      )
    }
  }

  fun onTerminalDelete(siteId: String, listener: (String) -> Unit): () -> Unit {
    val listeners = terminalListenersBySite.getOrPut(siteId) { mutableSetOf() }
    listeners += listener
    terminalNoteIdsBySite.value[siteId].orEmpty().forEach { notify(listener, it) }

    var disposed = false
    return {
      if (!disposed) {
        disposed = true
        listeners -= listener
        if (listeners.isEmpty()) terminalListenersBySite -= siteId
      }
    }
  }

  fun isTerminallyDeleted(siteId: String, noteId: String): Boolean =
    noteId in terminalNoteIdsBySite.value[siteId].orEmpty()

  private fun markTerminallyDeleted(siteId: String, noteId: String): Boolean {
    var added = false
    terminalNoteIdsBySite.update { noteIdsBySite ->
      val noteIds = noteIdsBySite[siteId].orEmpty()
      added = noteId !in noteIds
      if (added) noteIdsBySite + (siteId to (noteIds + noteId)) else noteIdsBySite
    }
    if (added) {
      terminalListenersBySite[siteId]?.toList()?.forEach { notify(it, noteId) }
    }
    return added
  }

  private fun notify(listener: (String) -> Unit, noteId: String) {
    try {
      listener(noteId)
    } catch (_: Throwable) {
      // Terminal observers are best-effort and must not interrupt state propagation.
    }
  }
}

internal fun Result<*, *>.isNoteNotFound(): Boolean =
  this is Result.Exception && exception is TypieError && exception.code == "not_found"
