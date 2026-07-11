package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue
import kotlinx.coroutines.async
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

private const val DOC_ID = "D1"

private fun TestScope.harness(): Pair<DocumentWsChannel, MutableList<FakeSyncWsSocket>> {
  val sockets = mutableListOf<FakeSyncWsSocket>()
  var ticketSeq = 0
  val connection =
    SyncWsConnection(
      socketFactory = {
        val socket = FakeSyncWsSocket()
        sockets.add(socket)
        socket
      },
      fetchTicket = { "TK-${++ticketSeq}" },
      scope = backgroundScope,
      pingIntervalMs = 120_000,
    )
  val channel = DocumentWsChannel(connection, DOC_ID, backgroundScope)
  return channel to sockets
}

private fun TestScope.handshake(socket: FakeSyncWsSocket) {
  runCurrent()
  socket.serverSend(WsServerMessage.HelloAck())
  runCurrent()
}

private fun chunk(rowId: String, seq: Int, offset: Int, bytes: ByteArray) =
  WsServerMessage.SnapshotChunk(
    documentId = DOC_ID,
    rowId = rowId,
    seq = seq,
    offset = offset,
    bytes = bytes,
  )

private fun snapshotEnd(seq: String = "1-0") =
  WsServerMessage.SnapshotEnd(
    documentId = DOC_ID,
    seq = seq,
    heads = ByteArray(0),
    durableHeads = ByteArray(0),
  )

class DocumentSnapshotLoaderTest {
  @Test
  fun accumulatesChunksInOrderAndReturnsOnSnapshotEnd() = runTest {
    val (channel, sockets) = harness()
    val result = backgroundScope.async { loadSnapshotBytes(channel) }
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1, 2)))
    socket.serverSend(chunk("B1", 1, 2, byteArrayOf(3)))
    socket.serverSend(snapshotEnd())
    runCurrent()

    assertContentEquals(byteArrayOf(1, 2, 3), result.await())
  }

  @Test
  fun restartClearsAccumulatedBufferBeforeReaccumulating() = runTest {
    val (channel, sockets) = harness()
    val result = backgroundScope.async { loadSnapshotBytes(channel) }
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(9, 9)))
    runCurrent()

    socket.serverSend(chunk("B1", 1, 5, byteArrayOf(8)))
    runCurrent()

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B2", 2, 0, byteArrayOf(1, 2)))
    socket.serverSend(snapshotEnd("2-0"))
    runCurrent()

    assertContentEquals(byteArrayOf(1, 2), result.await())
  }

  @Test
  fun permanentErrorThrowsSyncWsException() = runTest {
    val (channel, sockets) = harness()
    val result = backgroundScope.async { runCatching { loadSnapshotBytes(channel) } }
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(
      WsServerMessage.WsError(
        scope = "document",
        documentId = DOC_ID,
        code = "forbidden",
        permanent = true,
      )
    )
    runCurrent()

    val exception = assertIs<SyncWsException>(result.await().exceptionOrNull())
    assertEquals("forbidden", exception.code)
    assertTrue(exception.permanent)
  }
}
