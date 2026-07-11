package co.typie.editor.sync.ws

import co.typie.editor.sync.PullResult
import co.typie.editor.sync.PushResult
import co.typie.editor.sync.RemoteChangesetEvent
import co.typie.editor.sync.SyncTransport
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow

private class WsSyncTransportRestartSignal : Exception()

class WsSyncTransport(
  private val channel: DocumentWsChannel,
  private val connection: SyncWsConnection,
  private val documentId: String,
  private val onReload: suspend () -> Unit,
  private val scope: CoroutineScope,
) : SyncTransport {
  override suspend fun push(changesets: ByteArray): PushResult =
    connection.push(documentId, changesets)

  override suspend fun pull(sinceSeq: String?): PullResult = connection.pull(documentId, sinceSeq)

  override fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent> = flow {
    try {
      channel.events.collect { event ->
        when (event) {
          is AttachEvent.ChangesetsEvent ->
            emit(
              RemoteChangesetEvent(
                changesets = event.bundles,
                seq = event.seq,
                heads = event.heads,
                durableHeads = event.durableHeads,
              )
            )
          is AttachEvent.ReloadEvent -> throw WsSyncTransportRestartSignal()
          is AttachEvent.SnapshotRestart -> throw WsSyncTransportRestartSignal()
          else -> {}
        }
      }
    } catch (_: WsSyncTransportRestartSignal) {}
    onReload()
    awaitCancellation()
  }

  fun attach(): Flow<AttachEvent> = channel.freshSubscribe()
}
