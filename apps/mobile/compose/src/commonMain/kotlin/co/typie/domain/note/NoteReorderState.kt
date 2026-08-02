package co.typie.domain.note

import androidx.compose.runtime.Stable
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.resolveNextFractionalOrderMove
import co.typie.graphql.type.NoteStatus
import co.typie.result.Result
import co.typie.ui.component.reorder.ReorderDrop
import co.typie.ui.component.reorder.ReorderState
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.launch

internal data class NoteListIdentity(
  val siteId: String,
  val status: NoteStatus,
  val entityId: String? = null,
)

@Stable
internal class NoteReorderState(
  private val scope: CoroutineScope,
  private val reorderState: ReorderState<String>,
) {
  private val mutableFailures = MutableSharedFlow<Throwable>(extraBufferCapacity = 1)
  val failures = mutableFailures.asSharedFlow()

  private var identity: NoteListIdentity? = null
  private var authoritativeNotesById = emptyMap<String, NoteCard_note>()
  private var authoritativeOrders = emptyMap<String, String>()
  private var desired: List<String>? = null
  private var preferredKey: String? = null
  private var moveNote:
    (suspend (NoteCard_note, lowerOrder: String?, upperOrder: String?) -> Result<
        String,
        Nothing,
      >)? =
    null
  private var worker: Job? = null
  private var owner = 0

  fun sync(identity: NoteListIdentity, notes: List<NoteCard_note>) {
    val currentNotes =
      notes.filterNot { NoteSync.isTerminallyDeleted(identity.siteId, it.id) }.sortedBy { it.order }
    val nextNotesById = currentNotes.associateBy { it.id }
    val nextOrders = currentNotes.associate { it.id to it.order }

    if (this.identity != identity) {
      stopWorker()
      this.identity = identity
      authoritativeNotesById = nextNotesById
      authoritativeOrders = nextOrders
      reorderState.resetOrder(currentNotes.map { it.id })
      return
    }

    authoritativeNotesById = nextNotesById
    authoritativeOrders = nextOrders

    val desired = desired ?: return
    val authoritativeKeys = authoritativeKeys()
    if (desired.toSet() != authoritativeKeys.toSet()) {
      restoreAuthoritativeOrder()
    } else if (desired == authoritativeKeys) {
      this.desired = null
      preferredKey = null
    }
  }

  fun commit(
    drop: ReorderDrop<String>,
    moveNote:
      suspend (NoteCard_note, lowerOrder: String?, upperOrder: String?) -> Result<String, Nothing>,
  ) {
    desired = drop.orderedKeys
    preferredKey = drop.movedKey
    this.moveNote = moveNote
    if (worker?.isActive == true) return

    val operationOwner = owner
    worker = scope.launch {
      try {
        reconcile(operationOwner)
      } finally {
        if (worker === coroutineContext[Job]) {
          worker = null
        }
      }
    }
  }

  fun dispose() {
    restoreAuthoritativeOrder()
  }

  private suspend fun reconcile(operationOwner: Int) {
    while (operationOwner == owner) {
      val desired = desired ?: return
      val authoritativeKeys = authoritativeKeys()
      if (desired == authoritativeKeys) {
        this.desired = null
        preferredKey = null
        return
      }

      val currentPreferredKey = preferredKey
      val move =
        resolveNextFractionalOrderMove<String>(
          authoritativeOrders,
          desiredKeys = desired,
          preferredKey = currentPreferredKey,
        )
      val note = move?.key?.let(authoritativeNotesById::get)
      if (move == null || note == null) {
        fail(IllegalStateException("Cannot reconcile note order with the current server snapshot"))
        return
      }

      val result = moveNote?.invoke(note, move.lowerOrder, move.upperOrder)
      if (operationOwner != owner || result == null || this.desired == null) return
      val identity = identity ?: return
      if (NoteSync.isTerminallyDeleted(identity.siteId, note.id)) {
        restoreAuthoritativeOrder()
        return
      }

      when (result) {
        is Result.Ok -> {
          if (preferredKey == currentPreferredKey) preferredKey = null
          val previousKeys = authoritativeKeys()
          authoritativeOrders = authoritativeOrders + (note.id to result.value)
          authoritativeNotesById =
            authoritativeNotesById + (note.id to note.copy(order = result.value))
          if (authoritativeKeys() == previousKeys && this.desired == desired) {
            fail(
              IllegalStateException("Move note response did not advance the authoritative order")
            )
            return
          }
        }
        is Result.Exception -> {
          fail(result.exception)
          return
        }
        is Result.Err -> {
          fail(IllegalStateException("Unexpected note reorder error"))
          return
        }
      }
    }
  }

  private fun authoritativeKeys(): List<String> =
    authoritativeOrders.entries
      .sortedBy(Map.Entry<String, String>::value)
      .map(Map.Entry<String, String>::key)

  private fun fail(error: Throwable) {
    restoreAuthoritativeOrder()
    mutableFailures.tryEmit(error)
  }

  private fun restoreAuthoritativeOrder() {
    stopWorker()
    reorderState.resetOrder(authoritativeKeys())
  }

  private fun stopWorker() {
    owner += 1
    worker?.cancel()
    worker = null
    desired = null
    preferredKey = null
    moveNote = null
  }
}
