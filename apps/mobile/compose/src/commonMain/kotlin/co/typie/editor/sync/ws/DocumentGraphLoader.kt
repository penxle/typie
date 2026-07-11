package co.typie.editor.sync.ws

import co.typie.editor.ffi.GraphIngest

data class DocumentSyncBaseline(val seq: String, val heads: ByteArray, val durableHeads: ByteArray)

sealed interface DocumentGraphLoaderEvent {
  data class Loaded(
    val generation: Int,
    val handle: GraphIngest,
    val baseline: DocumentSyncBaseline,
  ) : DocumentGraphLoaderEvent

  data class Failed(val code: String) : DocumentGraphLoaderEvent
}

class DocumentGraphLoader(private val beginIngest: () -> GraphIngest) {
  private sealed interface State {
    data object Idle : State

    data class Receiving(val generation: Int, val handle: GraphIngest) : State

    data class Transferred(val generation: Int) : State

    data object Failed : State
  }

  private var state: State = State.Idle
  private var nextGeneration = 0

  fun handle(event: AttachEvent): DocumentGraphLoaderEvent? =
    when (event) {
      is AttachEvent.SnapshotChunkEvent -> onChunk(event.bytes)
      AttachEvent.SnapshotRestart -> onRestart()
      is AttachEvent.SnapshotEndEvent -> onEnd(event)
      AttachEvent.ReloadEvent -> onReload()
      is AttachEvent.PermanentErrorEvent -> onPermanentError(event)
      is AttachEvent.ChangesetsEvent -> null
    }

  fun cancel() {
    onReload()
  }

  private fun onChunk(bytes: ByteArray): DocumentGraphLoaderEvent? {
    val receiving = state as? State.Receiving
    val generation: Int
    val handle: GraphIngest
    if (receiving != null) {
      generation = receiving.generation
      handle = receiving.handle
    } else {
      generation = nextGeneration++
      handle = beginIngest()
    }
    handle.appendChunk(bytes)
    state = State.Receiving(generation, handle)
    return null
  }

  private fun onRestart(): DocumentGraphLoaderEvent? {
    val receiving = state as? State.Receiving ?: return null
    receiving.handle.abort()
    state = State.Receiving(receiving.generation, beginIngest())
    return null
  }

  private fun onEnd(event: AttachEvent.SnapshotEndEvent): DocumentGraphLoaderEvent? {
    val receiving = state as? State.Receiving ?: return null
    state = State.Transferred(receiving.generation)
    return DocumentGraphLoaderEvent.Loaded(
      generation = receiving.generation,
      handle = receiving.handle,
      baseline =
        DocumentSyncBaseline(
          seq = event.seq,
          heads = event.heads,
          durableHeads = event.durableHeads,
        ),
    )
  }

  private fun onReload(): DocumentGraphLoaderEvent? {
    val receiving = state as? State.Receiving ?: return null
    receiving.handle.abort()
    state = State.Idle
    return null
  }

  private fun onPermanentError(event: AttachEvent.PermanentErrorEvent): DocumentGraphLoaderEvent {
    (state as? State.Receiving)?.handle?.abort()
    state = State.Failed
    return DocumentGraphLoaderEvent.Failed(event.code)
  }
}
