package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
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
  }

private fun decodeServerMessageForTest(bytes: ByteArray): WsServerMessage? =
  decodeServerMessage(bytes)

class ProtocolTest {
  private val contractPushHex =
    "b90004617464707573686269646272316a646f63756d656e7449646244316a6368616e676573657473d84043010203"
  private val contractChangesetsHex =
    "b9000661746a6368616e6765736574736a646f63756d656e7449646244316373657163322d306762756e646c657382d840420102d8404103656865616473d840406c64757261626c654865616473d84040"

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
}
