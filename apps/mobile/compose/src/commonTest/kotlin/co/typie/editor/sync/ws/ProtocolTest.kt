package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.Serializable
import kotlinx.serialization.cbor.Cbor

@Serializable private data class TestProbe(val t: String)

@OptIn(ExperimentalSerializationApi::class)
private val testCbor = Cbor {
  ignoreUnknownKeys = true
  encodeDefaults = true
  alwaysUseByteString = true
}

private fun decodeClientMessageForTest(bytes: ByteArray): WsClientMessage? {
  val t =
    try {
      syncCbor.decodeFromByteArray(TestProbe.serializer(), bytes).t
    } catch (_: Exception) {
      return null
    }
  return try {
    when (t) {
      "hello" -> syncCbor.decodeFromByteArray(WsClientMessage.Hello.serializer(), bytes)
      "ping" -> syncCbor.decodeFromByteArray(WsClientMessage.Ping.serializer(), bytes)
      "attach" -> syncCbor.decodeFromByteArray(WsClientMessage.Attach.serializer(), bytes)
      "detach" -> syncCbor.decodeFromByteArray(WsClientMessage.Detach.serializer(), bytes)
      "push" -> syncCbor.decodeFromByteArray(WsClientMessage.Push.serializer(), bytes)
      "pull" -> syncCbor.decodeFromByteArray(WsClientMessage.Pull.serializer(), bytes)
      "asset-pull" -> syncCbor.decodeFromByteArray(WsClientMessage.AssetPull.serializer(), bytes)
      "asset-heartbeat" ->
        syncCbor.decodeFromByteArray(WsClientMessage.AssetHeartbeat.serializer(), bytes)
      "asset-failed" ->
        syncCbor.decodeFromByteArray(WsClientMessage.AssetFailed.serializer(), bytes)
      else -> null
    }
  } catch (_: Exception) {
    null
  }
}

private fun encodeServerMessageForTest(message: WsServerMessage): ByteArray =
  when (message) {
    is WsServerMessage.HelloAck ->
      testCbor.encodeToByteArray(WsServerMessage.HelloAck.serializer(), message)
    is WsServerMessage.Pong ->
      testCbor.encodeToByteArray(WsServerMessage.Pong.serializer(), message)
    is WsServerMessage.AttachAck ->
      testCbor.encodeToByteArray(WsServerMessage.AttachAck.serializer(), message)
    is WsServerMessage.SnapshotChunk ->
      testCbor.encodeToByteArray(WsServerMessage.SnapshotChunk.serializer(), message)
    is WsServerMessage.SnapshotEnd ->
      testCbor.encodeToByteArray(WsServerMessage.SnapshotEnd.serializer(), message)
    is WsServerMessage.Changesets ->
      testCbor.encodeToByteArray(WsServerMessage.Changesets.serializer(), message)
    is WsServerMessage.Reload ->
      testCbor.encodeToByteArray(WsServerMessage.Reload.serializer(), message)
    is WsServerMessage.PushAck ->
      testCbor.encodeToByteArray(WsServerMessage.PushAck.serializer(), message)
    is WsServerMessage.PullAck ->
      testCbor.encodeToByteArray(WsServerMessage.PullAck.serializer(), message)
    is WsServerMessage.WsError ->
      testCbor.encodeToByteArray(WsServerMessage.WsError.serializer(), message)
    is WsServerMessage.AssetState ->
      testCbor.encodeToByteArray(WsServerMessage.AssetState.serializer(), message)
    is WsServerMessage.AssetChanged ->
      testCbor.encodeToByteArray(WsServerMessage.AssetChanged.serializer(), message)
  }

private fun decodeServerMessageForTest(bytes: ByteArray): WsServerMessage? =
  decodeServerMessage(bytes)

class ProtocolTest {
  private val contractPushHex =
    "b90004617464707573686269646272316a646f63756d656e7449646244316a6368616e676573657473d84043010203"
  private val contractChangesetsHex =
    "b9000661746a6368616e6765736574736a646f63756d656e7449646244316373657163322d306762756e646c657382d840420102d8404103656865616473d840406c64757261626c654865616473d84040"

  // { t: 'asset-pull', documentId: 'D1', requestId: 'R1', ids: ['A1', 'A2'] }
  private val contractAssetPullHex =
    "b9000461746a61737365742d70756c6c6a646f63756d656e744964624431697265717565737449646252316369647382624131624132"

  // { t: 'asset-heartbeat', documentId: 'D1', items: [{ id: 'A1', nonce: 'N1' }] }
  private val contractAssetHeartbeatHex =
    "b9000361746f61737365742d6865617274626561746a646f63756d656e744964624431656974656d7381b90002626964624131656e6f6e6365624e31"

  // { t: 'asset-failed', documentId: 'D1', items: [{ id: 'A1', nonce: 'N1' }] }
  private val contractAssetFailedHex =
    "b9000361746c61737365742d6661696c65646a646f63756d656e744964624431656974656d7381b90002626964624131656e6f6e6365624e31"

  // 5 entries: missing / pending / ready-image(no placeholder) / ready-image(placeholder) /
  // ready-file(size=1073741824). See task-9-report.md for the exact node -e generation command.
  private val contractAssetStateHex =
    "b9000561746b61737365742d73746174656a646f63756d656e744964624431697265717565737449646252316661737365747385b90002626964624130657374617465676d697373696e67b900036269646241316573746174656770656e64696e67646d657461b90003646b696e6465696d616765646e616d6565612e706e676473697a65193039b90003626964624132657374617465657265616479656173736574b90007647479706565696d6167656269646241326375726c6d68747470733a2f2f782f696d676b6f726967696e616c55726c6e68747470733a2f2f782f6f726967657769647468190320666865696768741902586b706c616365686f6c646572f6b90003626964624133657374617465657265616479656173736574b90007647479706565696d6167656269646241336375726c6e68747470733a2f2f782f696d67336b6f726967696e616c55726c6f68747470733a2f2f782f6f7269673365776964746818646668656967687418326b706c616365686f6c646572684241534536345048b90003626964624134657374617465657265616479656173736574b9000564747970656466696c656269646241346375726c6e68747470733a2f2f782f66696c65646e616d6567646f632e7064666473697a651a400000006566696e616cf5"

  // { t: 'asset-changed', documentId: 'D1', ids: ['A1', 'A2'] }
  private val contractAssetChangedHex =
    "b9000361746d61737365742d6368616e6765646a646f63756d656e7449646244316369647382624131624132"

  // { t: 'asset-teleport', documentId: 'D1', foo: 'bar' }
  private val contractUnknownTypeHex =
    "b9000361746e61737365742d74656c65706f72746a646f63756d656e74496462443163666f6f63626172"

  // asset-state with an extra top-level field and an extra nested field inside `asset`.
  private val contractAssetStateWithFutureFieldsHex =
    "b9000661746b61737365742d73746174656a646f63756d656e744964624431697265717565737449646252316661737365747381b90003626964624135657374617465657265616479656173736574b90008647479706565696d6167656269646241356375726c6c68747470733a2f2f782f69356b6f726967696e616c55726c6c68747470733a2f2f782f6f356577696474680a66686569676874146b706c616365686f6c646572f6716675747572654e65737465644669656c646769676e6f7265646566696e616cf46e667574757265546f704669656c646769676e6f726564"

  private fun hexToBytes(hex: String): ByteArray =
    ByteArray(hex.length / 2) { i -> hex.substring(i * 2, i * 2 + 2).toInt(16).toByte() }

  @Test
  fun decodesContractPushVector() {
    val decodedBack = decodeServerMessageForTest(hexToBytes(contractPushHex))
    assertNull(decodedBack)
    val client = decodeClientMessageForTest(hexToBytes(contractPushHex))
    assertIs<WsClientMessage.Push>(client)
    assertEquals("r1", client.id)
    assertEquals("D1", client.documentId)
    assertContentEquals(byteArrayOf(1, 2, 3), client.changesets)
  }

  @Test
  fun encodeMatchesContractVectorOrRoundTrips() {
    val encoded =
      encodeClientMessage(
        WsClientMessage.Push(id = "r1", documentId = "D1", changesets = byteArrayOf(1, 2, 3))
      )
    val reDecoded = decodeClientMessageForTest(encoded)
    assertIs<WsClientMessage.Push>(reDecoded)
    assertContentEquals(byteArrayOf(1, 2, 3), reDecoded.changesets)
  }

  @Test
  fun optionalAbsentFieldsAreOmittedFromEncoding() {
    val encoded = encodeClientMessage(WsClientMessage.Attach(documentId = "D1"))
    val decoded = decodeClientMessageForTest(encoded)
    assertIs<WsClientMessage.Attach>(decoded)
    assertNull(decoded.sinceSeq)
    assertNull(decoded.snapshotCursor)
    val text = encoded.joinToString("") { (it.toInt() and 0xff).toString(16).padStart(2, '0') }
    check(
      !text.contains(
        "sinceSeq".toByteArray().joinToString("") {
          (it.toInt() and 0xff).toString(16).padStart(2, '0')
        }
      )
    ) {
      "absent sinceSeq must not be encoded"
    }
  }

  @Test
  fun decodesServerMessagesAndIgnoresUnknown() {
    val pong = decodeServerMessage(encodeServerMessageForTest(WsServerMessage.Pong()))
    assertIs<WsServerMessage.Pong>(pong)
    assertNull(decodeServerMessage(byteArrayOf(0xff.toByte(), 0x00)))
  }

  @Test
  fun decodesContractChangesetsVectorWithByteStringList() {
    val decoded = decodeServerMessage(hexToBytes(contractChangesetsHex))
    assertIs<WsServerMessage.Changesets>(decoded)
    assertEquals(2, decoded.bundles.size)
    assertContentEquals(byteArrayOf(1, 2), decoded.bundles[0])
    assertContentEquals(byteArrayOf(3), decoded.bundles[1])
  }

  @Test
  fun byteArrayFieldsRoundTripAsByteStrings() {
    val ack =
      decodeServerMessage(
        encodeServerMessageForTest(
          WsServerMessage.PushAck(id = "r1", heads = byteArrayOf(9), durableHeads = ByteArray(0))
        )
      )
    assertIs<WsServerMessage.PushAck>(ack)
    assertContentEquals(byteArrayOf(9), ack.heads)
  }

  @Test
  fun compareStreamSeqComparesNumerically() {
    check(compareStreamSeq("2-0", "10-0") < 0)
    check(compareStreamSeq("10-2", "10-10") < 0)
    assertEquals(0, compareStreamSeq("10-1", "10-1"))
  }

  @OptIn(ExperimentalStdlibApi::class)
  @Test
  fun helloAlwaysEncodesCapabilities() {
    val hex =
      encodeClientMessage(WsClientMessage.Hello(ticket = "TK", clientId = "C1")).toHexString()
    assertEquals(
      "bf61746568656c6c6f667469636b657462544b68636c69656e7449646243316c6361706162696c69746965739fffff",
      hex,
    )
  }

  @Test
  fun decodesContractAssetPullVectorAndEncodeRoundTrips() {
    val decoded = decodeClientMessageForTest(hexToBytes(contractAssetPullHex))
    assertIs<WsClientMessage.AssetPull>(decoded)
    assertEquals("D1", decoded.documentId)
    assertEquals("R1", decoded.requestId)
    assertEquals(listOf("A1", "A2"), decoded.ids)

    val encoded =
      encodeClientMessage(
        WsClientMessage.AssetPull(documentId = "D1", requestId = "R1", ids = listOf("A1", "A2"))
      )
    val reDecoded = decodeClientMessageForTest(encoded)
    assertIs<WsClientMessage.AssetPull>(reDecoded)
    assertEquals(decoded, reDecoded)
  }

  @Test
  fun decodesContractAssetHeartbeatVectorAndEncodeRoundTrips() {
    val decoded = decodeClientMessageForTest(hexToBytes(contractAssetHeartbeatHex))
    assertIs<WsClientMessage.AssetHeartbeat>(decoded)
    assertEquals("D1", decoded.documentId)
    assertEquals(listOf(WsAssetItem(id = "A1", nonce = "N1")), decoded.items)

    val encoded =
      encodeClientMessage(
        WsClientMessage.AssetHeartbeat(
          documentId = "D1",
          items = listOf(WsAssetItem(id = "A1", nonce = "N1")),
        )
      )
    val reDecoded = decodeClientMessageForTest(encoded)
    assertIs<WsClientMessage.AssetHeartbeat>(reDecoded)
    assertEquals(decoded, reDecoded)
  }

  @Test
  fun decodesContractAssetFailedVectorAndEncodeRoundTrips() {
    val decoded = decodeClientMessageForTest(hexToBytes(contractAssetFailedHex))
    assertIs<WsClientMessage.AssetFailed>(decoded)
    assertEquals("D1", decoded.documentId)
    assertEquals(listOf(WsAssetItem(id = "A1", nonce = "N1")), decoded.items)

    val encoded =
      encodeClientMessage(
        WsClientMessage.AssetFailed(
          documentId = "D1",
          items = listOf(WsAssetItem(id = "A1", nonce = "N1")),
        )
      )
    val reDecoded = decodeClientMessageForTest(encoded)
    assertIs<WsClientMessage.AssetFailed>(reDecoded)
    assertEquals(decoded, reDecoded)
  }

  @Test
  fun decodesContractAssetStateVectorWithThreeStates() {
    val decoded = decodeServerMessage(hexToBytes(contractAssetStateHex))
    assertIs<WsServerMessage.AssetState>(decoded)
    assertEquals("D1", decoded.documentId)
    assertEquals("R1", decoded.requestId)
    assertTrue(decoded.final)
    assertEquals(5, decoded.assets.size)

    val missing = decoded.assets[0]
    assertEquals("A0", missing.id)
    assertEquals("missing", missing.state)
    assertNull(missing.asset)
    assertNull(missing.meta)

    val pending = decoded.assets[1]
    assertEquals("A1", pending.id)
    assertEquals("pending", pending.state)
    assertNull(pending.asset)
    val meta = pending.meta
    assertIs<WsPendingMeta>(meta)
    assertEquals("image", meta.kind)
    assertEquals("a.png", meta.name)
    assertEquals(12345L, meta.size)

    val readyImage = decoded.assets[2]
    assertEquals("A2", readyImage.id)
    assertEquals("ready", readyImage.state)
    assertNull(readyImage.meta)
    val image = readyImage.asset
    assertIs<WsReadyAsset>(image)
    assertEquals("image", image.type)
    assertEquals("A2", image.id)
    assertEquals("https://x/img", image.url)
    assertEquals("https://x/orig", image.originalUrl)
    assertEquals(800L, image.width)
    assertEquals(600L, image.height)
    assertNull(image.placeholder)
    assertNull(image.name)
    assertNull(image.size)

    val readyImageWithPlaceholder = decoded.assets[3]
    val imageWithPlaceholder = readyImageWithPlaceholder.asset
    assertIs<WsReadyAsset>(imageWithPlaceholder)
    assertEquals("BASE64PH", imageWithPlaceholder.placeholder)

    val readyFile = decoded.assets[4]
    assertEquals("A4", readyFile.id)
    assertEquals("ready", readyFile.state)
    val file = readyFile.asset
    assertIs<WsReadyAsset>(file)
    assertEquals("file", file.type)
    assertEquals("https://x/file", file.url)
    assertEquals("doc.pdf", file.name)
    assertEquals(1_073_741_824L, file.size)
    assertNull(file.width)
    assertNull(file.height)
  }

  @Test
  fun decodesContractAssetChangedVector() {
    val decoded = decodeServerMessage(hexToBytes(contractAssetChangedHex))
    assertIs<WsServerMessage.AssetChanged>(decoded)
    assertEquals("D1", decoded.documentId)
    assertEquals(listOf("A1", "A2"), decoded.ids)
  }

  @Test
  fun unknownServerMessageTypeDecodesToNull() {
    assertNull(decodeServerMessage(hexToBytes(contractUnknownTypeHex)))
  }

  @Test
  fun unknownFieldsAtTopLevelAndNestedAreIgnoredNotFatal() {
    val decoded = decodeServerMessage(hexToBytes(contractAssetStateWithFutureFieldsHex))
    assertIs<WsServerMessage.AssetState>(decoded)
    assertFalse(decoded.final)
    val entry = decoded.assets.single()
    assertEquals("A5", entry.id)
    val asset = entry.asset
    assertIs<WsReadyAsset>(asset)
    assertEquals("https://x/i5", asset.url)
    assertNull(asset.placeholder)
  }
}
