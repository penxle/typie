package co.typie.editor.sync.ws

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.Serializable
import kotlinx.serialization.cbor.Cbor

@Serializable private data class FakeSyncWsProbe(val t: String)

@OptIn(ExperimentalSerializationApi::class)
private val fakeSyncWsCbor = Cbor {
  ignoreUnknownKeys = true
  encodeDefaults = true
  alwaysUseByteString = true
}

private fun decodeClientMessageForFake(bytes: ByteArray): WsClientMessage? {
  val t =
    try {
      fakeSyncWsCbor.decodeFromByteArray(FakeSyncWsProbe.serializer(), bytes).t
    } catch (_: Exception) {
      return null
    }
  return try {
    when (t) {
      "hello" -> fakeSyncWsCbor.decodeFromByteArray(WsClientMessage.Hello.serializer(), bytes)
      "ping" -> fakeSyncWsCbor.decodeFromByteArray(WsClientMessage.Ping.serializer(), bytes)
      "attach" -> fakeSyncWsCbor.decodeFromByteArray(WsClientMessage.Attach.serializer(), bytes)
      "detach" -> fakeSyncWsCbor.decodeFromByteArray(WsClientMessage.Detach.serializer(), bytes)
      "push" -> fakeSyncWsCbor.decodeFromByteArray(WsClientMessage.Push.serializer(), bytes)
      "pull" -> fakeSyncWsCbor.decodeFromByteArray(WsClientMessage.Pull.serializer(), bytes)
      else -> null
    }
  } catch (_: Exception) {
    null
  }
}

private fun encodeServerMessageForFake(message: WsServerMessage): ByteArray =
  when (message) {
    is WsServerMessage.HelloAck ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.HelloAck.serializer(), message)
    is WsServerMessage.Pong ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.Pong.serializer(), message)
    is WsServerMessage.AttachAck ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.AttachAck.serializer(), message)
    is WsServerMessage.SnapshotChunk ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.SnapshotChunk.serializer(), message)
    is WsServerMessage.SnapshotEnd ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.SnapshotEnd.serializer(), message)
    is WsServerMessage.Changesets ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.Changesets.serializer(), message)
    is WsServerMessage.Reload ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.Reload.serializer(), message)
    is WsServerMessage.PushAck ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.PushAck.serializer(), message)
    is WsServerMessage.PullAck ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.PullAck.serializer(), message)
    is WsServerMessage.WsError ->
      fakeSyncWsCbor.encodeToByteArray(WsServerMessage.WsError.serializer(), message)
  }

internal fun clientMessageTypeOf(message: WsClientMessage): String =
  when (message) {
    is WsClientMessage.Hello -> "hello"
    is WsClientMessage.Ping -> "ping"
    is WsClientMessage.Attach -> "attach"
    is WsClientMessage.Detach -> "detach"
    is WsClientMessage.Push -> "push"
    is WsClientMessage.Pull -> "pull"
  }

internal fun serverMessageTypeOf(message: WsServerMessage): String =
  when (message) {
    is WsServerMessage.HelloAck -> "hello-ack"
    is WsServerMessage.Pong -> "pong"
    is WsServerMessage.AttachAck -> "attach-ack"
    is WsServerMessage.SnapshotChunk -> "snapshot-chunk"
    is WsServerMessage.SnapshotEnd -> "snapshot-end"
    is WsServerMessage.Changesets -> "changesets"
    is WsServerMessage.Reload -> "reload"
    is WsServerMessage.PushAck -> "push-ack"
    is WsServerMessage.PullAck -> "pull-ack"
    is WsServerMessage.WsError -> "error"
  }

internal class FakeSyncWsSocket : SyncWsSocket {
  val sent = mutableListOf<WsClientMessage>()
  private val closedDeferred = CompletableDeferred<Int>()
  private val incomingFlow = MutableSharedFlow<ByteArray>(extraBufferCapacity = 64)

  override val incoming: Flow<ByteArray> = incomingFlow
  override val closed: Deferred<Int> = closedDeferred

  override suspend fun send(bytes: ByteArray) {
    decodeClientMessageForFake(bytes)?.let { sent.add(it) }
  }

  override fun close() {
    closedDeferred.complete(1000)
  }

  fun serverSend(message: WsServerMessage) {
    incomingFlow.tryEmit(encodeServerMessageForFake(message))
  }

  fun serverClose(code: Int) {
    closedDeferred.complete(code)
  }

  fun lastOf(t: String): WsClientMessage? = sent.lastOrNull { clientMessageTypeOf(it) == t }
}
