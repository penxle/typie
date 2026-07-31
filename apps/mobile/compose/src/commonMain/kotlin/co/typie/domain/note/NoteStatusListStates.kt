package co.typie.domain.note

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.type.NoteStatus

@Stable
internal class NoteStatusListStates(initialVisibleStatus: NoteStatus = NoteStatus.OPEN) {
  var visibleStatus by mutableStateOf(initialVisibleStatus.asListStatus())
    private set

  private val states =
    mapOf(
      NoteStatus.OPEN to NoteListState(NoteStatus.OPEN),
      NoteStatus.RESOLVED to NoteListState(NoteStatus.RESOLVED),
    )
  private var serverNotes: List<NoteCard_note>? = null

  fun state(status: NoteStatus): NoteListState = states.getValue(status.asListStatus())

  fun sync(notes: List<NoteCard_note>) {
    serverNotes = notes
    states.forEach { (status, state) ->
      if (status == visibleStatus) {
        state.sync(notes)
      } else {
        state.settle(notes)
      }
    }
  }

  fun updateVisibleStatus(status: NoteStatus) {
    val nextStatus = status.asListStatus()
    if (visibleStatus == nextStatus) return

    visibleStatus = nextStatus
    serverNotes?.let { notes -> states.values.forEach { it.settle(notes) } }
  }

  fun convergeDeletedNote(noteId: String, fallbackNote: (status: NoteStatus) -> NoteCard_note?) {
    val currentNotes = serverNotes
    val remainingNotes = currentNotes?.filterNot { it.id == noteId }
    states.forEach { (status, state) ->
      if (status == visibleStatus) {
        state.markDeleted(
          noteId = noteId,
          fallbackNote =
            currentNotes?.firstOrNull { it.id == noteId && it.status == status }
              ?: fallbackNote(status),
        )
      } else if (remainingNotes != null) {
        state.settle(remainingNotes)
      }
    }
    serverNotes = remainingNotes
  }
}

private fun NoteStatus.asListStatus(): NoteStatus =
  when (this) {
    NoteStatus.RESOLVED -> NoteStatus.RESOLVED
    else -> NoteStatus.OPEN
  }
