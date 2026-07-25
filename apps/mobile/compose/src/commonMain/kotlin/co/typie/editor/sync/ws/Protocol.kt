package co.typie.editor.sync.ws

import kotlinx.serialization.EncodeDefault
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.Serializable
import kotlinx.serialization.cbor.Cbor

const val SYNC_WS_SUBPROTOCOL = "typie-sync.v1"

// 서버는 0건·상한 초과 asset 메시지를 malformed로 보고 소켓을 끊는다 — 그 소켓에 붙은 모든 문서가
// 스냅샷을 다시 받아야 한다. 호출측 청킹이 어긋나도 프레임 하나가 잘릴 뿐 연결은 살아 있어야 한다.
const val SYNC_WS_ASSET_MESSAGE_MAX_ITEMS = 100

class SyncWsException(val code: String, val permanent: Boolean) :
  Exception("sync ws request failed: $code")

fun compareStreamSeq(a: String, b: String): Int {
  val (am, an) = a.split("-").map { it.toLong() }
  val (bm, bn) = b.split("-").map { it.toLong() }
  if (am != bm) return if (am < bm) -1 else 1
  if (an != bn) return if (an < bn) -1 else 1
  return 0
}

@Serializable data class WsSnapshotCursor(val rowId: String, val seq: Int, val offset: Int)

@Serializable data class WsAssetItem(val id: String, val nonce: String)

@Serializable data class WsPendingMeta(val kind: String, val name: String, val size: Long)

// 네 종류(image/file/embed/archived)의 payload를 평탄화한 것이라 type별로 존재하는 필드가 다르다.
// archived는 url조차 없으므로 전부 nullable이어야 디코드가 깨지지 않는다.
@Serializable
data class WsReadyAsset(
  val type: String,
  val id: String,
  val url: String? = null,
  val originalUrl: String? = null,
  val width: Long? = null,
  val height: Long? = null,
  val placeholder: String? = null,
  val name: String? = null,
  val size: Long? = null,
  val title: String? = null,
  val description: String? = null,
  val thumbnailUrl: String? = null,
  val html: String? = null,
  val content: String? = null,
)

@Serializable
data class WsAssetStateEntry(
  val id: String,
  val state: String,
  val asset: WsReadyAsset? = null,
  val meta: WsPendingMeta? = null,
)

@OptIn(ExperimentalSerializationApi::class)
sealed interface WsClientMessage {
  @Serializable
  data class Hello(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "hello",
    val ticket: String,
    val clientId: String,
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val capabilities: List<String> = emptyList(),
  ) : WsClientMessage

  @Serializable
  data class Ping(@EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "ping") :
    WsClientMessage

  @Serializable
  data class Attach(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "attach",
    val documentId: String,
    @EncodeDefault(EncodeDefault.Mode.NEVER) val sinceSeq: String? = null,
    @EncodeDefault(EncodeDefault.Mode.NEVER) val snapshotCursor: WsSnapshotCursor? = null,
  ) : WsClientMessage

  @Serializable
  data class Detach(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "detach",
    val documentId: String,
  ) : WsClientMessage

  @Serializable
  data class Push(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "push",
    val id: String,
    val documentId: String,
    val changesets: ByteArray,
  ) : WsClientMessage

  @Serializable
  data class Pull(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "pull",
    val id: String,
    val documentId: String,
    @EncodeDefault(EncodeDefault.Mode.NEVER) val sinceSeq: String? = null,
  ) : WsClientMessage

  @Serializable
  data class AssetPull(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "asset-pull",
    val documentId: String,
    val requestId: String,
    val ids: List<String>,
  ) : WsClientMessage

  @Serializable
  data class AssetHeartbeat(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "asset-heartbeat",
    val documentId: String,
    val items: List<WsAssetItem>,
  ) : WsClientMessage

  @Serializable
  data class AssetFailed(
    @EncodeDefault(EncodeDefault.Mode.ALWAYS) val t: String = "asset-failed",
    val documentId: String,
    val items: List<WsAssetItem>,
  ) : WsClientMessage
}

@OptIn(ExperimentalSerializationApi::class)
sealed interface WsServerMessage {
  @Serializable
  data class HelloAck(val t: String = "hello-ack", val capabilities: List<String> = emptyList()) :
    WsServerMessage

  @Serializable data class Pong(val t: String = "pong") : WsServerMessage

  @Serializable
  data class AttachAck(val t: String = "attach-ack", val documentId: String) : WsServerMessage

  @Serializable
  data class SnapshotChunk(
    val t: String = "snapshot-chunk",
    val documentId: String,
    val rowId: String,
    val seq: Int,
    val offset: Int,
    val bytes: ByteArray,
  ) : WsServerMessage

  @Serializable
  data class SnapshotEnd(
    val t: String = "snapshot-end",
    val documentId: String,
    val seq: String,
    val heads: ByteArray,
    val durableHeads: ByteArray,
  ) : WsServerMessage

  @Serializable
  data class Changesets(
    val t: String = "changesets",
    val documentId: String,
    val seq: String,
    val bundles: List<ByteArray>,
    val heads: ByteArray,
    val durableHeads: ByteArray,
  ) : WsServerMessage

  @Serializable
  data class Reload(val t: String = "reload", val documentId: String) : WsServerMessage

  @Serializable
  data class PushAck(
    val t: String = "push-ack",
    val id: String,
    val heads: ByteArray,
    val durableHeads: ByteArray,
  ) : WsServerMessage

  @Serializable
  data class PullAck(
    val t: String = "pull-ack",
    val id: String,
    val changesets: List<ByteArray>,
    val seq: String,
    val heads: ByteArray,
    val durableHeads: ByteArray,
    val needsReload: Boolean,
  ) : WsServerMessage

  @Serializable
  data class WsError(
    val t: String = "error",
    val scope: String,
    @EncodeDefault(EncodeDefault.Mode.NEVER) val documentId: String? = null,
    @EncodeDefault(EncodeDefault.Mode.NEVER) val id: String? = null,
    val code: String,
    val permanent: Boolean,
  ) : WsServerMessage

  @Serializable
  data class AssetState(
    val t: String = "asset-state",
    val documentId: String,
    val requestId: String,
    val assets: List<WsAssetStateEntry>,
    val final: Boolean,
  ) : WsServerMessage

  @Serializable
  data class AssetChanged(
    val t: String = "asset-changed",
    val documentId: String,
    val ids: List<String>,
  ) : WsServerMessage
}

@OptIn(ExperimentalSerializationApi::class)
internal val syncCbor = Cbor {
  ignoreUnknownKeys = true
  alwaysUseByteString = true
}

@Serializable private data class Probe(val t: String)

fun encodeClientMessage(message: WsClientMessage): ByteArray =
  when (message) {
    is WsClientMessage.Hello ->
      syncCbor.encodeToByteArray(WsClientMessage.Hello.serializer(), message)
    is WsClientMessage.Ping ->
      syncCbor.encodeToByteArray(WsClientMessage.Ping.serializer(), message)
    is WsClientMessage.Attach ->
      syncCbor.encodeToByteArray(WsClientMessage.Attach.serializer(), message)
    is WsClientMessage.Detach ->
      syncCbor.encodeToByteArray(WsClientMessage.Detach.serializer(), message)
    is WsClientMessage.Push ->
      syncCbor.encodeToByteArray(WsClientMessage.Push.serializer(), message)
    is WsClientMessage.Pull ->
      syncCbor.encodeToByteArray(WsClientMessage.Pull.serializer(), message)
    is WsClientMessage.AssetPull ->
      syncCbor.encodeToByteArray(WsClientMessage.AssetPull.serializer(), message)
    is WsClientMessage.AssetHeartbeat ->
      syncCbor.encodeToByteArray(WsClientMessage.AssetHeartbeat.serializer(), message)
    is WsClientMessage.AssetFailed ->
      syncCbor.encodeToByteArray(WsClientMessage.AssetFailed.serializer(), message)
  }

fun decodeServerMessage(bytes: ByteArray): WsServerMessage? {
  val t =
    try {
      syncCbor.decodeFromByteArray(Probe.serializer(), bytes).t
    } catch (_: Exception) {
      return null
    }
  return try {
    when (t) {
      "hello-ack" -> syncCbor.decodeFromByteArray(WsServerMessage.HelloAck.serializer(), bytes)
      "pong" -> syncCbor.decodeFromByteArray(WsServerMessage.Pong.serializer(), bytes)
      "attach-ack" -> syncCbor.decodeFromByteArray(WsServerMessage.AttachAck.serializer(), bytes)
      "snapshot-chunk" ->
        syncCbor.decodeFromByteArray(WsServerMessage.SnapshotChunk.serializer(), bytes)
      "snapshot-end" ->
        syncCbor.decodeFromByteArray(WsServerMessage.SnapshotEnd.serializer(), bytes)
      "changesets" -> syncCbor.decodeFromByteArray(WsServerMessage.Changesets.serializer(), bytes)
      "reload" -> syncCbor.decodeFromByteArray(WsServerMessage.Reload.serializer(), bytes)
      "push-ack" -> syncCbor.decodeFromByteArray(WsServerMessage.PushAck.serializer(), bytes)
      "pull-ack" -> syncCbor.decodeFromByteArray(WsServerMessage.PullAck.serializer(), bytes)
      "error" -> syncCbor.decodeFromByteArray(WsServerMessage.WsError.serializer(), bytes)
      "asset-state" -> syncCbor.decodeFromByteArray(WsServerMessage.AssetState.serializer(), bytes)
      "asset-changed" ->
        syncCbor.decodeFromByteArray(WsServerMessage.AssetChanged.serializer(), bytes)
      else -> null
    }
  } catch (_: Exception) {
    null
  }
}
