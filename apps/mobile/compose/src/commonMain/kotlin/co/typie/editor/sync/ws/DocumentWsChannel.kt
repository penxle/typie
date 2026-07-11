package co.typie.editor.sync.ws

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.channelFlow
import kotlinx.coroutines.launch

sealed interface AttachEvent {
  data class SnapshotChunkEvent(
    val rowId: String,
    val seq: Int,
    val offset: Int,
    val bytes: ByteArray,
  ) : AttachEvent

  data object SnapshotRestart : AttachEvent

  data class SnapshotEndEvent(val seq: String, val heads: ByteArray, val durableHeads: ByteArray) :
    AttachEvent

  data class ChangesetsEvent(
    val seq: String,
    val bundles: List<ByteArray>,
    val heads: ByteArray,
    val durableHeads: ByteArray,
  ) : AttachEvent

  data object ReloadEvent : AttachEvent

  data class PermanentErrorEvent(val code: String) : AttachEvent
}

class DocumentWsChannel(
  private val connection: SyncWsConnection,
  private val documentId: String,
  private val scope: CoroutineScope,
  private val onEvicted: () -> Unit = {},
) {
  private companion object {
    const val RETRY_BASE_MS = 1_000L
    const val RETRY_CAP_MS = 30_000L
  }

  private val _events =
    MutableSharedFlow<AttachEvent>(
      replay = 0,
      extraBufferCapacity = 256,
      onBufferOverflow = BufferOverflow.SUSPEND,
    )
  val events: SharedFlow<AttachEvent> = _events.asSharedFlow()

  private val internalEvents = Channel<AttachEvent>(Channel.UNLIMITED)

  private val channelScope =
    CoroutineScope(scope.coroutineContext + SupervisorJob(scope.coroutineContext[Job]))

  private var awaitingAck = false
  private var cursor: WsSnapshotCursor? = null
  private var probe: WsSnapshotCursor? = null
  private var snapshotDone = false
  private var liveSeq: String? = null
  private var retryAttempts = 0
  private var retryJob: Job? = null
  private var permanentlyFailed = false

  private val unregisterChannelHandler =
    connection.registerChannel(documentId) { message -> handleMessage(message) }
  private val unregisterReconnectedHandler = connection.onReconnected { handleReconnected() }

  init {
    channelScope.launch {
      for (event in internalEvents) {
        _events.emit(event)
      }
    }
    channelScope.launch {
      var subscribed = false
      _events.subscriptionCount.collect { count ->
        if (count > 0 && !subscribed) {
          subscribed = true
          onFirstSubscriberAttached()
        } else if (count == 0 && subscribed) {
          subscribed = false
          onLastSubscriberDetached()
        }
      }
    }
  }

  fun freshSubscribe(): Flow<AttachEvent> = channelFlow {
    launch(start = CoroutineStart.UNDISPATCHED) {
      events.collect { send(it) }
    }
    scope.launch {
      if (cursor != null || snapshotDone) {
        restartGeneration()
      }
    }
  }

  private fun handleMessage(message: WsServerMessage) {
    when (message) {
      is WsServerMessage.AttachAck -> handleAttachAck()
      is WsServerMessage.SnapshotChunk -> handleSnapshotChunk(message)
      is WsServerMessage.SnapshotEnd -> handleSnapshotEnd(message)
      is WsServerMessage.Changesets -> handleChangesets(message)
      is WsServerMessage.Reload -> handleReload()
      is WsServerMessage.WsError -> handleError(message)
      else -> {}
    }
  }

  private fun handleAttachAck() {
    awaitingAck = false
  }

  private fun handleSnapshotChunk(message: WsServerMessage.SnapshotChunk) {
    if (awaitingAck) return
    val activeProbe = probe
    if (activeProbe != null) {
      probe = null
      if (message.rowId == activeProbe.rowId && message.offset == activeProbe.offset) {
        appendChunk(message)
        return
      }
      emitEvent(AttachEvent.SnapshotRestart)
      cursor = null
    }
    val current = cursor
    val accepted =
      if (current == null) {
        message.offset == 0
      } else {
        (message.rowId == current.rowId && message.offset == current.offset) ||
          (message.rowId != current.rowId && message.offset == 0)
      }
    if (!accepted) {
      restartGeneration()
      return
    }
    appendChunk(message)
  }

  private fun appendChunk(message: WsServerMessage.SnapshotChunk) {
    cursor =
      WsSnapshotCursor(
        rowId = message.rowId,
        seq = message.seq,
        offset = message.offset + message.bytes.size,
      )
    emitEvent(
      AttachEvent.SnapshotChunkEvent(
        rowId = message.rowId,
        seq = message.seq,
        offset = message.offset,
        bytes = message.bytes,
      )
    )
  }

  private fun handleSnapshotEnd(message: WsServerMessage.SnapshotEnd) {
    if (awaitingAck) return
    retryAttempts = 0
    cursor = null
    probe = null
    snapshotDone = true
    liveSeq = message.seq.ifEmpty { null }
    emitEvent(
      AttachEvent.SnapshotEndEvent(
        seq = message.seq,
        heads = message.heads,
        durableHeads = message.durableHeads,
      )
    )
  }

  private fun handleChangesets(message: WsServerMessage.Changesets) {
    if (awaitingAck) return
    val seq = message.seq
    val baseline = liveSeq
    val passes = seq.isEmpty() || baseline == null || compareStreamSeq(seq, baseline) > 0
    if (!passes) return
    if (seq.isNotEmpty()) liveSeq = seq
    emitEvent(
      AttachEvent.ChangesetsEvent(
        seq = seq,
        bundles = message.bundles,
        heads = message.heads,
        durableHeads = message.durableHeads,
      )
    )
  }

  private fun handleReload() {
    emitEvent(AttachEvent.ReloadEvent)
    beginFreshGeneration()
  }

  private fun handleError(message: WsServerMessage.WsError) {
    if (message.permanent) {
      permanentlyFailed = true
      retryJob?.cancel()
      retryJob = null
      emitEvent(AttachEvent.PermanentErrorEvent(message.code))
      return
    }
    if (cursor != null) {
      emitEvent(AttachEvent.SnapshotRestart)
    }
    cursor = null
    probe = null
    scheduleRetry()
  }

  private fun scheduleRetry() {
    if (retryJob != null || permanentlyFailed) return
    retryAttempts += 1
    val delayMs = minOf(RETRY_BASE_MS * (1L shl (retryAttempts - 1).coerceAtMost(16)), RETRY_CAP_MS)
    retryJob = scope.launch {
      delay(delayMs)
      retryJob = null
      if (_events.subscriptionCount.value > 0) reattach()
    }
  }

  private fun handleReconnected() {
    if (_events.subscriptionCount.value == 0) return
    reattach()
  }

  private fun reattach() {
    awaitingAck = true
    val snapshotCursor = cursor
    when {
      snapshotDone -> connection.sendAttach(documentId, liveSeq ?: "", null)
      snapshotCursor != null -> {
        probe = snapshotCursor
        connection.sendAttach(documentId, null, snapshotCursor)
      }
      else -> connection.sendAttach(documentId, null, null)
    }
  }

  private fun restartGeneration() {
    emitEvent(AttachEvent.SnapshotRestart)
    beginFreshGeneration()
  }

  private fun beginFreshGeneration() {
    cursor = null
    probe = null
    snapshotDone = false
    awaitingAck = true
    connection.sendDetach(documentId)
    connection.sendAttach(documentId, null, null)
  }

  private fun onFirstSubscriberAttached() {
    reattach()
  }

  private fun onLastSubscriberDetached() {
    connection.sendDetach(documentId)
    cursor = null
    probe = null
    snapshotDone = false
    liveSeq = null
    awaitingAck = false
    retryJob?.cancel()
    retryJob = null
    retryAttempts = 0
    permanentlyFailed = false
    unregisterChannelHandler()
    unregisterReconnectedHandler()
    onEvicted()
    channelScope.cancel()
  }

  private fun emitEvent(event: AttachEvent) {
    internalEvents.trySend(event)
  }
}
