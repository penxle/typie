package co.typie.domain.note

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.type.NoteStatus

@Stable
internal class NoteListState(private val status: NoteStatus) {
  var hasSettled by mutableStateOf(false)
    private set

  private val enteringNotesById = mutableStateMapOf<String, NoteCard_note>()
  private val enteringAnimationIds = mutableStateMapOf<String, Boolean>()
  private val expectedEntryNotesById = mutableStateMapOf<String, NoteCard_note>()
  private val exitingNotesById = mutableStateMapOf<String, ExitingNoteSnapshot>()

  fun sync(serverNotes: List<NoteCard_note>) {
    hasSettled = true
    val serverIds = serverNotes.mapTo(mutableSetOf()) { it.id }

    enteringNotesById.keys
      .filter { it in serverIds }
      .forEach { noteId -> enteringNotesById.remove(noteId) }

    expectedEntryNotesById.keys.toList().forEach { noteId ->
      if (noteId in serverIds) {
        enteringAnimationIds[noteId] = true
        expectedEntryNotesById.remove(noteId)
      }
    }

    exitingNotesById.keys.toList().forEach { noteId ->
      val snapshot = exitingNotesById[noteId] ?: return@forEach
      if (!snapshot.isVisible && noteId !in serverIds) {
        exitingNotesById.remove(noteId)
      }
    }
  }

  fun merge(serverNotes: List<NoteCard_note>): List<NoteCard_note> {
    val mergedNotesById = linkedMapOf<String, NoteCard_note>()
    val exitingNoteIds = exitingNotesById.keys
    val serverIds = serverNotes.mapTo(mutableSetOf()) { it.id }

    enteringNotesById.values
      .asSequence()
      .filter { it.status == status && it.id !in serverIds && it.id !in exitingNoteIds }
      .forEach { mergedNotesById[it.id] = it }

    serverNotes
      .asSequence()
      .filter { it.id !in exitingNoteIds }
      .forEach { mergedNotesById[it.id] = it }

    exitingNotesById.values
      .asSequence()
      .filter { it.isVisible }
      .forEach { mergedNotesById[it.note.id] = it.note }

    return mergedNotesById.values.sortedBy { it.order }
  }

  fun markEntering(note: NoteCard_note) {
    expectedEntryNotesById.remove(note.id)
    exitingNotesById.remove(note.id)
    enteringNotesById[note.id] = note
    enteringAnimationIds[note.id] = true
  }

  fun expectEntry(note: NoteCard_note) {
    if (note.status != status) {
      return
    }

    exitingNotesById.remove(note.id)
    expectedEntryNotesById[note.id] = note
  }

  fun finishEntering(noteId: String) {
    enteringAnimationIds.remove(noteId)
  }

  fun markExiting(note: NoteCard_note) {
    expectedEntryNotesById.remove(note.id)
    enteringNotesById.remove(note.id)
    enteringAnimationIds.remove(note.id)
    exitingNotesById[note.id] = ExitingNoteSnapshot(note = note, isVisible = true)
  }

  fun finishExiting(noteId: String) {
    val snapshot = exitingNotesById[noteId] ?: return
    exitingNotesById[noteId] = snapshot.copy(isVisible = false)
  }

  fun remove(noteId: String) {
    expectedEntryNotesById.remove(noteId)
    enteringNotesById.remove(noteId)
    enteringAnimationIds.remove(noteId)
    exitingNotesById.remove(noteId)
  }

  fun isEntering(noteId: String): Boolean = noteId in enteringAnimationIds

  fun isExiting(noteId: String): Boolean = noteId in exitingNotesById

  fun isExitVisible(noteId: String): Boolean = exitingNotesById[noteId]?.isVisible == true
}

private data class ExitingNoteSnapshot(val note: NoteCard_note, val isVisible: Boolean)
