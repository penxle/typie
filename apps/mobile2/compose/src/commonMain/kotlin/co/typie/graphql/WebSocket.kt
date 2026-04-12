package co.typie.graphql

import co.typie.network.Http
import com.apollographql.apollo.api.http.HttpHeader
import com.apollographql.apollo.network.websocket.WebSocket
import com.apollographql.apollo.network.websocket.WebSocketEngine
import com.apollographql.apollo.network.websocket.WebSocketListener
import io.ktor.client.plugins.websocket.webSocket
import io.ktor.client.request.headers
import io.ktor.client.request.url
import io.ktor.http.HttpMethod
import io.ktor.websocket.CloseReason
import io.ktor.websocket.Frame
import io.ktor.websocket.readBytes
import io.ktor.websocket.readText
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch

object WebSocketSession {
  suspend fun create(): String =
    Apollo.executeMutation(WebSocket_CreateWsSession_Mutation()).createWsSession
}

object KtorWebSocketEngine : WebSocketEngine {
  private val coroutineScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

  override fun newWebSocket(
    url: String,
    headers: List<HttpHeader>,
    listener: WebSocketListener,
  ): WebSocket =
    object : WebSocket {
      private val sendFrameChannel = Channel<Frame>(Channel.UNLIMITED)

      init {
        coroutineScope.launch {
          try {
            Http.webSocket({
              method = HttpMethod.Get
              url(url)
              headers { headers.forEach { append(it.name, it.value) } }
            }) {
              listener.onOpen()

              launch {
                while (true) {
                  send(sendFrameChannel.receive())
                }
              }

              while (true) {
                when (val frame = incoming.receive()) {
                  is Frame.Text -> listener.onMessage(frame.readText())
                  is Frame.Binary -> listener.onMessage(frame.readBytes())
                  is Frame.Close -> {
                    val reason = closeReason.await()
                    listener.onClosed(reason?.code?.toInt(), reason?.message)
                    sendFrameChannel.close()
                  }

                  else -> {}
                }
              }
            }
          } catch (e: Exception) {
            listener.onClosed(code = 1006, reason = "Network error: ${e.message}")
            sendFrameChannel.close()
          }
        }
      }

      override fun send(text: String) {
        sendFrameChannel.trySend(Frame.Text(text))
      }

      override fun send(data: ByteArray) {
        sendFrameChannel.trySend(Frame.Binary(true, data))
      }

      override fun close(code: Int, reason: String) {
        sendFrameChannel.trySend(Frame.Close(CloseReason(code.toShort(), reason)))
        sendFrameChannel.close()
      }
    }

  override fun close() {}
}
