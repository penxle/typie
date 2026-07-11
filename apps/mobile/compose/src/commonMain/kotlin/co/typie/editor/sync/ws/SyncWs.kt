package co.typie.editor.sync.ws

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
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch

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
        throw e
      } catch (_: Throwable) {}
      val reason = session.closeReason.await()
      closedDeferred.complete(
        SyncWsSocketClosed(code = reason?.code?.toInt() ?: 1000, reason = reason?.message ?: "")
      )
    }
  }

  override suspend fun send(bytes: ByteArray) {
    session.send(Frame.Binary(true, bytes))
  }

  override fun close() {
    scope.launch { runCatching { session.close() } }
  }

  companion object {
    suspend fun connect(url: String, scope: CoroutineScope): SyncWsSocket {
      val session =
        Http.webSocketSession(url) { header(HttpHeaders.SecWebSocketProtocol, SYNC_WS_SUBPROTOCOL) }
      return KtorSyncWsSocket(session, scope)
    }
  }
}

object SyncWs {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

  val connection: SyncWsConnection by lazy {
    SyncWsConnection(
      socketFactory = { KtorSyncWsSocket.connect(url = "${Konfig.WS_URL}/sync", scope = scope) },
      fetchTicket = { WebSocketSession.create() },
      scope = scope,
    )
  }

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
}
