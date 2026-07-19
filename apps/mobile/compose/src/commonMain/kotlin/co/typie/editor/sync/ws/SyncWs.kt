package co.typie.editor.sync.ws

import co.touchlab.kermit.Logger
import co.typie.Konfig
import co.typie.graphql.WebSocketSession
import co.typie.network.Http
import io.ktor.client.plugins.websocket.DefaultClientWebSocketSession
import io.ktor.client.plugins.websocket.webSocketSession
import io.ktor.client.request.header
import io.ktor.http.HttpHeaders
import io.ktor.websocket.Frame
import io.ktor.websocket.close
import io.ktor.websocket.readBytes
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

private class KtorSyncWsSocket
private constructor(
  private val session: DefaultClientWebSocketSession,
  private val scope: CoroutineScope,
) : SyncWsSocket {
  private val closedDeferred = CompletableDeferred<SyncWsSocketClosed>()
  private val incomingFlow = MutableSharedFlow<ByteArray>(extraBufferCapacity = 256)

  override val incoming: Flow<ByteArray> = incomingFlow
  override val closed: Deferred<SyncWsSocketClosed> = closedDeferred

  init {
    scope.launch {
      try {
        for (frame in session.incoming) {
          if (frame is Frame.Binary) incomingFlow.emit(frame.readBytes())
        }
      } catch (e: CancellationException) {
        if (!isActive) throw e
      } catch (_: Throwable) {}
      val reason =
        try {
          withTimeoutOrNull(CLOSE_REASON_TIMEOUT_MS) { session.closeReason.await() }
        } catch (e: CancellationException) {
          if (!isActive) throw e
          null
        } catch (_: Throwable) {
          null
        }
      closedDeferred.complete(
        SyncWsSocketClosed(code = reason?.code?.toInt() ?: 1000, reason = reason?.message ?: "")
      )
    }
  }

  override suspend fun send(bytes: ByteArray) {
    session.send(Frame.Binary(true, bytes))
  }

  override fun close() {
    scope.launch {
      runCatching { session.close() }
      if (withTimeoutOrNull(CLOSE_GRACE_MS) { closedDeferred.await() } == null) terminate()
    }
  }

  override fun terminate() {
    closedDeferred.complete(SyncWsSocketClosed(code = 1006, reason = "terminated"))
    session.cancel()
  }

  companion object {
    private const val CLOSE_GRACE_MS = 5_000L
    private const val CLOSE_REASON_TIMEOUT_MS = 5_000L

    suspend fun connect(url: String, scope: CoroutineScope): SyncWsSocket {
      val session =
        Http.webSocketSession(url) { header(HttpHeaders.SecWebSocketProtocol, SYNC_WS_SUBPROTOCOL) }
      return KtorSyncWsSocket(session, scope)
    }
  }
}

internal class SyncWsUncaughtException(cause: Throwable) : RuntimeException(cause)

object SyncWs {
  private val exceptionHandler = CoroutineExceptionHandler { _, e ->
    Logger.e(e) { "SyncWs: uncaught exception" }
    Sentry.captureException(SyncWsUncaughtException(e))
  }
  private val scope =
    CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate + exceptionHandler)

  /**
   * WS 재연결(onReconnected) 시 호출된다. 앱 계층(RootShell)이 SubscriptionService.refresh를 건다. 이 var를 세팅해도
   * lazy 연결은 초기화되지 않으며, 연결 생성 시점에 콜백이 배선된다.
   */
  var onSyncReconnected: (() -> Unit)? = null

  private val connectionDelegate = lazy {
    SyncWsConnection(
        socketFactory = { KtorSyncWsSocket.connect(url = "${Konfig.WS_URL}/sync", scope = scope) },
        fetchTicket = { WebSocketSession.create() },
        scope = scope,
      )
      .also { conn -> conn.onReconnected { onSyncReconnected?.invoke() } }
  }
  val connection: SyncWsConnection by connectionDelegate

  private val channels = mutableMapOf<String, DocumentWsChannel>()

  fun channel(documentId: String): DocumentWsChannel =
    channels.getOrPut(documentId) {
      lateinit var created: DocumentWsChannel
      created =
        DocumentWsChannel(connection, documentId, scope) {
          if (channels[documentId] === created) channels.remove(documentId)
        }
      created
    }

  fun retryDocument(documentId: String) {
    channels.remove(documentId)
    connection.resetTerminal()
  }

  /** 재구독 전환 시 연결 terminal + 열린 채널의 permanent 실패를 방어적으로 리셋·재attach 한다. */
  fun resetPermanentlyFailedChannels() {
    if (!connectionDelegate.isInitialized()) return
    connection.resetTerminal()
    channels.values.toList().forEach { it.resetPermanentFailure() }
  }

  fun onAppForeground() {
    if (connectionDelegate.isInitialized()) connection.onAppForeground()
  }
}
