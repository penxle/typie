package co.typie.editor.sync

import co.touchlab.kermit.Logger
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class RemoteChangesetPipeline(
  private val editor: SyncEditor,
  private val headsSink: SyncHeadsSink,
  private val transport: SyncTransport,
  initialSeq: String,
  private val scope: CoroutineScope,
  private val onNeedsReload: suspend () -> Unit,
) {
  private companion object {
    const val POLL_INTERVAL_MS = 10_000L
    const val RECONNECT_DELAY_MS = 1000L
    const val RECONNECT_CAP_MS = 30_000L
  }

  private var syncSeq = initialSeq
  private var subscriptionJob: Job? = null
  private var pollJob: Job? = null
  private val applyMutex = Mutex()

  fun start() {
    if (subscriptionJob != null || pollJob != null) return
    subscriptionJob =
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        var attempts = 0
        while (isActive) {
          try {
            transport.subscribe(syncSeq.ifEmpty { null }).collect { event ->
              attempts = 0
              apply(event.changesets, event.seq, event.heads, event.durableHeads)
            }
          } catch (e: CancellationException) {
            throw e
          } catch (e: Throwable) {
            if (isPermanentSyncError(e)) {
              Logger.w(e) {
                "RemoteChangesetPipeline: permanent subscription error, giving up (polling remains)"
              }
              return@launch
            }
            Logger.w(e) { "RemoteChangesetPipeline: subscription failed, reconnecting" }
          }
          attempts += 1
          delay(minOf(RECONNECT_DELAY_MS * attempts, RECONNECT_CAP_MS))
        }
      }
    pollJob = scope.launch {
      while (isActive) {
        delay(POLL_INTERVAL_MS)
        refetchFromServer()
      }
    }
  }

  suspend fun refetchFromServer() {
    try {
      val result = transport.pull(syncSeq.ifEmpty { null })
      if (result.needsReload) {
        onNeedsReload()
        stop()
        return
      }
      apply(result.changesets, result.seq, result.heads, result.durableHeads)
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      Logger.w(e) { "RemoteChangesetPipeline: pull failed" }
    }
  }

  private suspend fun apply(
    changesets: List<ByteArray>,
    seq: String,
    heads: ByteArray,
    durableHeads: ByteArray,
  ) {
    applyMutex.withLock {
      for (payload in changesets) {
        if (payload.isNotEmpty()) {
          editor.receiveRemoteChangeset(payload)
        }
      }
      if (seq.isNotEmpty()) syncSeq = seq
      headsSink.setConfirmedHeads(heads)
      headsSink.setDurableHeads(durableHeads)
    }
  }

  fun stop() {
    subscriptionJob?.cancel()
    subscriptionJob = null
    pollJob?.cancel()
    pollJob = null
  }
}
