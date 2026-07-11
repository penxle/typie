package co.typie.editor.sync.ws

import co.typie.editor.sync.concatChangesets
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.onEach

suspend fun loadDocumentSnapshotBytes(documentId: String): ByteArray =
  loadSnapshotBytes(SyncWs.channel(documentId))

internal suspend fun loadSnapshotBytes(channel: DocumentWsChannel): ByteArray {
  val chunks = mutableListOf<ByteArray>()
  channel
    .freshSubscribe()
    .onEach { event ->
      when (event) {
        is AttachEvent.SnapshotChunkEvent -> chunks.add(event.bytes)
        AttachEvent.SnapshotRestart,
        AttachEvent.ReloadEvent -> chunks.clear()
        is AttachEvent.PermanentErrorEvent -> throw SyncWsException(event.code, permanent = true)
        is AttachEvent.SnapshotEndEvent,
        is AttachEvent.ChangesetsEvent -> {}
      }
    }
    .first { it is AttachEvent.SnapshotEndEvent }
  return chunks.concatChangesets()
}
