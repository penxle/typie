package co.typie.editor.sync.ws

import co.touchlab.kermit.Logger
import co.typie.editor.sync.PullResult
import co.typie.editor.sync.PushResult
import io.sentry.kotlin.multiplatform.Sentry
import kotlin.concurrent.Volatile
import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

data class SyncWsSocketClosed(val code: Int, val reason: String)

interface SyncWsSocket {
  suspend fun send(bytes: ByteArray)

  fun close()

  val incoming: Flow<ByteArray>
  val closed: Deferred<SyncWsSocketClosed>
}

typealias SocketFactory = suspend () -> SyncWsSocket

@OptIn(ExperimentalUuidApi::class)
class SyncWsConnection(
  private val socketFactory: SocketFactory,
  private val fetchTicket: suspend () -> String,
  private val scope: CoroutineScope,
  private val pingIntervalMs: Long = 30_000,
  private val reconnectBaseMs: Long = 1_000,
  private val idleTimeoutMs: Long = 60_000,
) {
  private companion object {
    const val RECONNECT_CAP_MS = 30_000L
    const val PING_MAX_MISSES = 2
    const val PROTOCOL_ERROR_CLOSE_CODE = 4003
    const val AUTH_FAILED_CLOSE_CODE = 4001
    const val AUTH_FAILED_MAX_STREAK = 3
  }

  private val clientId = Uuid.random().toString()
  private val connectMutex = Mutex()

  @Volatile private var socket: SyncWsSocket? = null

  @Volatile private var ready = false

  @Volatile private var disposed = false

  @Volatile private var terminal = false
  @Volatile private var terminalError: SyncWsException? = null

  private var attempts = 0
  private var requestSeq = 0
  private var missedPongs = 0
  private var authFailedStreak = 0
  private var lastCloseCode: Int? = null

  private val pending = mutableMapOf<String, CompletableDeferred<WsServerMessage>>()
  private val channelHandlers = mutableMapOf<String, MutableList<(WsServerMessage) -> Unit>>()
  private val reconnectedCallbacks = mutableListOf<() -> Unit>()

  private var sendChannel = Channel<WsClientMessage>(Channel.UNLIMITED)
  private var helloAckDeferred: CompletableDeferred<Unit>? = null
  private var receiverJob: Job? = null
  private var senderJob: Job? = null
  private var closedWatcherJob: Job? = null
  private var pingJob: Job? = null
  private var idleJob: Job? = null
  private var reconnectJob: Job? = null

  suspend fun push(documentId: String, changesets: ByteArray): PushResult {
    val response = request { id ->
      WsClientMessage.Push(id = id, documentId = documentId, changesets = changesets)
    }
    val ack =
      response as? WsServerMessage.PushAck ?: throw SyncWsException("unexpected_response", false)
    return PushResult(heads = ack.heads, durableHeads = ack.durableHeads)
  }

  suspend fun pull(documentId: String, sinceSeq: String?): PullResult {
    val response = request { id ->
      WsClientMessage.Pull(id = id, documentId = documentId, sinceSeq = sinceSeq)
    }
    val ack =
      response as? WsServerMessage.PullAck ?: throw SyncWsException("unexpected_response", false)
    return PullResult(
      changesets = ack.changesets,
      seq = ack.seq,
      heads = ack.heads,
      durableHeads = ack.durableHeads,
      needsReload = ack.needsReload,
    )
  }

  fun registerChannel(documentId: String, handler: (WsServerMessage) -> Unit): () -> Unit {
    if (terminal && documentId !in channelHandlers) resetTerminal()
    val handlers = channelHandlers.getOrPut(documentId) { mutableListOf() }
    handlers.add(handler)
    recomputeIdle()
    return {
      channelHandlers[documentId]?.let { existing ->
        existing.remove(handler)
        if (existing.isEmpty()) channelHandlers.remove(documentId)
      }
      recomputeIdle()
    }
  }

  fun sendAttach(documentId: String, sinceSeq: String?, snapshotCursor: WsSnapshotCursor?) {
    if (!ready) {
      ensureConnected()
      return
    }
    sendChannel.trySend(
      WsClientMessage.Attach(
        documentId = documentId,
        sinceSeq = sinceSeq,
        snapshotCursor = snapshotCursor,
      )
    )
  }

  fun sendDetach(documentId: String) {
    if (!ready) return
    sendChannel.trySend(WsClientMessage.Detach(documentId = documentId))
  }

  fun onReconnected(callback: () -> Unit): () -> Unit {
    reconnectedCallbacks.add(callback)
    return { reconnectedCallbacks.remove(callback) }
  }

  fun dispose() {
    if (disposed) return
    disposed = true
    reconnectJob?.cancel()
    reconnectJob = null
    idleJob?.cancel()
    idleJob = null
    stopPing()
    senderJob?.cancel()
    senderJob = null
    receiverJob?.cancel()
    receiverJob = null
    closedWatcherJob?.cancel()
    closedWatcherJob = null
    socket?.close()
    socket = null
    ready = false
    failAllPending()
  }

  private suspend fun request(build: (String) -> WsClientMessage): WsServerMessage {
    if (disposed) throw SyncWsException("connection_lost", false)
    terminalError?.let { throw it }
    val id = "r${++requestSeq}"
    val message = build(id)
    val deferred = CompletableDeferred<WsServerMessage>()
    pending[id] = deferred
    recomputeIdle()
    sendChannel.trySend(message)
    ensureConnected()
    return deferred.await()
  }

  private fun ensureConnected() {
    if (disposed || terminal || socket != null || reconnectJob != null) return
    scope.launch {
      connectMutex.withLock {
        if (disposed || socket != null) return@withLock
        connect()
      }
    }
  }

  private suspend fun connect() {
    val ticket =
      try {
        fetchTicket()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.w(e) { "SyncWsConnection: ticket fetch failed" }
        scheduleReconnect()
        return
      }
    if (disposed) return

    val newSocket =
      try {
        socketFactory()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.w(e) { "SyncWsConnection: socket connect failed" }
        scheduleReconnect()
        return
      }
    if (disposed) {
      newSocket.close()
      return
    }

    socket = newSocket
    ready = false
    missedPongs = 0
    val helloAck = CompletableDeferred<Unit>()
    helloAckDeferred = helloAck

    receiverJob = scope.launch {
      newSocket.incoming.collect { bytes ->
        decodeServerMessage(bytes)?.let { dispatch(it, newSocket) }
      }
    }

    closedWatcherJob = scope.launch {
      val closed = newSocket.closed.await()
      onSocketClosed(newSocket, closed)
    }

    senderJob = scope.launch {
      try {
        newSocket.send(
          encodeClientMessage(WsClientMessage.Hello(ticket = ticket, clientId = clientId))
        )
        helloAck.await()
        for (message in sendChannel) {
          newSocket.send(encodeClientMessage(message))
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.w(e) { "SyncWsConnection: send failed" }
        newSocket.close()
      }
    }
  }

  private fun dispatch(message: WsServerMessage, sourceSocket: SyncWsSocket) {
    if (sourceSocket !== socket) return
    when (message) {
      is WsServerMessage.HelloAck -> handleHelloAck()
      is WsServerMessage.Pong -> missedPongs = 0
      is WsServerMessage.PushAck -> resolvePending(message.id, message)
      is WsServerMessage.PullAck -> resolvePending(message.id, message)
      is WsServerMessage.WsError -> handleError(message)
      is WsServerMessage.AttachAck -> routeToChannel(message.documentId, message)
      is WsServerMessage.SnapshotChunk -> routeToChannel(message.documentId, message)
      is WsServerMessage.SnapshotEnd -> routeToChannel(message.documentId, message)
      is WsServerMessage.Changesets -> routeToChannel(message.documentId, message)
      is WsServerMessage.Reload -> routeToChannel(message.documentId, message)
    }
  }

  private fun handleHelloAck() {
    ready = true
    attempts = 0
    authFailedStreak = 0
    helloAckDeferred?.complete(Unit)
    startPing()
    recomputeIdle()
    reconnectedCallbacks.toList().forEach { it() }
  }

  private fun handleError(message: WsServerMessage.WsError) {
    if (message.scope == "request") {
      val id = message.id ?: return
      rejectPending(id, SyncWsException(message.code, message.permanent))
      return
    }
    routeToChannel(message.documentId, message)
  }

  private fun resolvePending(id: String, message: WsServerMessage) {
    pending.remove(id)?.complete(message)
    recomputeIdle()
  }

  private fun rejectPending(id: String, error: SyncWsException) {
    pending.remove(id)?.completeExceptionally(error)
    recomputeIdle()
  }

  private fun routeToChannel(documentId: String?, message: WsServerMessage) {
    if (documentId == null) return
    channelHandlers[documentId]?.toList()?.forEach { it(message) }
  }

  private fun onSocketClosed(sourceSocket: SyncWsSocket, closed: SyncWsSocketClosed) {
    if (sourceSocket !== socket) return
    Logger.w { "SyncWsConnection: socket closed (${closed.code} ${closed.reason})" }
    lastCloseCode = closed.code
    socket = null
    ready = false
    helloAckDeferred = null
    stopPing()
    idleJob?.cancel()
    idleJob = null
    senderJob?.cancel()
    senderJob = null
    receiverJob?.cancel()
    receiverJob = null
    closedWatcherJob = null
    sendChannel = Channel(Channel.UNLIMITED)

    val permanent =
      when (closed.code) {
        PROTOCOL_ERROR_CLOSE_CODE -> true
        AUTH_FAILED_CLOSE_CODE -> {
          authFailedStreak += 1
          authFailedStreak >= AUTH_FAILED_MAX_STREAK
        }
        else -> {
          authFailedStreak = 0
          false
        }
      }

    if (permanent) {
      enterTerminal(closed)
      return
    }

    failAllPending()
    if (!disposed) scheduleReconnect()
  }

  private fun enterTerminal(closed: SyncWsSocketClosed) {
    if (terminal) return
    terminal = true
    reconnectJob?.cancel()
    reconnectJob = null

    val code = permanentCodeFor(closed.code)
    val error = SyncWsException(code, permanent = true)
    terminalError = error

    Logger.e {
      "SyncWsConnection: connection permanent after close (${closed.code} ${closed.reason})"
    }
    Sentry.captureException(error) { scope ->
      scope.setTag("ws_close_code", closed.code.toString())
      scope.setExtra("ws_close_reason", closed.reason)
    }

    failAllPendingPermanent(error)
    propagatePermanentToChannels(error)
  }

  fun resetTerminal() {
    terminal = false
    terminalError = null
    authFailedStreak = 0
  }

  private fun permanentCodeFor(closeCode: Int): String =
    if (closeCode == PROTOCOL_ERROR_CLOSE_CODE) {
      "connection_permanent_protocol_error"
    } else {
      "connection_permanent_auth_failed"
    }

  private fun failAllPending() {
    val entries = pending.values.toList()
    pending.clear()
    for (entry in entries) entry.completeExceptionally(SyncWsException("connection_lost", false))
  }

  private fun failAllPendingPermanent(error: SyncWsException) {
    val entries = pending.values.toList()
    pending.clear()
    for (entry in entries) entry.completeExceptionally(error)
  }

  private fun propagatePermanentToChannels(error: SyncWsException) {
    for (documentId in channelHandlers.keys.toList()) {
      routeToChannel(
        documentId,
        WsServerMessage.WsError(
          scope = "document",
          documentId = documentId,
          code = error.code,
          permanent = true,
        ),
      )
    }
  }

  private fun scheduleReconnect() {
    if (disposed || reconnectJob != null) return
    if (pending.isEmpty() && channelHandlers.isEmpty()) return
    attempts += 1
    val delayMs =
      minOf(reconnectBaseMs * (1L shl (attempts - 1).coerceAtMost(16)), RECONNECT_CAP_MS)
    Logger.w {
      "SyncWsConnection: reconnecting (attempt $attempts) in ${delayMs}ms after close $lastCloseCode"
    }
    reconnectJob = scope.launch {
      delay(delayMs)
      reconnectJob = null
      ensureConnected()
    }
  }

  private fun startPing() {
    missedPongs = 0
    pingJob?.cancel()
    pingJob = scope.launch {
      while (true) {
        delay(pingIntervalMs)
        if (missedPongs >= PING_MAX_MISSES) {
          socket?.close()
          break
        }
        missedPongs += 1
        sendChannel.trySend(WsClientMessage.Ping())
      }
    }
  }

  private fun stopPing() {
    pingJob?.cancel()
    pingJob = null
    missedPongs = 0
  }

  private fun recomputeIdle() {
    idleJob?.cancel()
    idleJob = null
    if (disposed || !ready) return
    if (pending.isEmpty() && channelHandlers.isEmpty()) {
      idleJob = scope.launch {
        delay(idleTimeoutMs)
        idleJob = null
        socket?.close()
      }
    }
  }
}
