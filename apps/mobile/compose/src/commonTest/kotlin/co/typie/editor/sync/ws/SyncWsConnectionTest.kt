package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.async
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

private data class ConnectionHarness(
  val connection: SyncWsConnection,
  val sockets: MutableList<FakeSyncWsSocket>,
)

private fun TestScope.harness(
  pingIntervalMs: Long = 30_000,
  reconnectBaseMs: Long = 1_000,
  idleTimeoutMs: Long = 60_000,
): ConnectionHarness {
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
      scope = CoroutineScope(coroutineContext),
      pingIntervalMs = pingIntervalMs,
      reconnectBaseMs = reconnectBaseMs,
      idleTimeoutMs = idleTimeoutMs,
    )
  return ConnectionHarness(connection, sockets)
}

// NOTE: never use advanceUntilIdle() in this file once a socket may be ready — the ping loop
// is a `while (true) { delay(pingIntervalMs); ... }` recurring timer, so advanceUntilIdle()
// (which advances virtual time until the task queue is empty) never returns. Use runCurrent()
// for zero-delay suspension chains, and explicit advanceTimeBy()+runCurrent() to cross real delays.
private fun TestScope.handshake(socket: FakeSyncWsSocket) {
  runCurrent()
  socket.serverSend(WsServerMessage.HelloAck())
  runCurrent()
}

class SyncWsConnectionTest {
  @Test
  fun lazyConnectSendsTicketThenHelloThenPushInOrder() = runTest {
    val (connection, sockets) = harness()
    val pushDeferred = async { connection.push("D1", byteArrayOf(1)) }
    runCurrent()
    assertEquals(1, sockets.size)
    val socket = sockets[0]
    val hello = assertIs<WsClientMessage.Hello>(socket.lastOf("hello"))
    assertEquals("TK-1", hello.ticket)
    assertNull(socket.lastOf("push"))

    socket.serverSend(WsServerMessage.HelloAck())
    runCurrent()

    val push = assertIs<WsClientMessage.Push>(socket.lastOf("push"))
    assertEquals("D1", push.documentId)
    socket.serverSend(
      WsServerMessage.PushAck(id = push.id, heads = byteArrayOf(9), durableHeads = byteArrayOf(8))
    )

    val result = pushDeferred.await()
    assertContentEquals(byteArrayOf(9), result.heads)
    assertContentEquals(byteArrayOf(8), result.durableHeads)
    connection.dispose()
  }

  @Test
  fun requestErrorWithPermanentScopeThrowsSyncWsException() = runTest {
    val (connection, sockets) = harness()
    val pushResult = async { runCatching { connection.push("D1", byteArrayOf(1)) } }
    runCurrent()
    val socket = sockets[0]
    handshake(socket)

    val push = assertIs<WsClientMessage.Push>(socket.lastOf("push"))
    socket.serverSend(
      WsServerMessage.WsError(
        scope = "request",
        id = push.id,
        code = "invalid_changeset_payload",
        permanent = true,
      )
    )

    val error = assertIs<SyncWsException>(pushResult.await().exceptionOrNull())
    assertEquals("invalid_changeset_payload", error.code)
    assertTrue(error.permanent)
    connection.dispose()
  }

  @Test
  fun disconnectRejectsPendingAndReconnectFetchesNewTicket() = runTest {
    val (connection, sockets) = harness()
    val pushResult = async { runCatching { connection.push("D1", byteArrayOf(1)) } }
    runCurrent()
    val socket0 = sockets[0]
    handshake(socket0)

    socket0.serverClose(1006)
    val error = assertIs<SyncWsException>(pushResult.await().exceptionOrNull())
    assertEquals("connection_lost", error.code)
    assertFalse(error.permanent)

    val pushDeferred2 = async { connection.push("D1", byteArrayOf(2)) }
    runCurrent()
    assertEquals(2, sockets.size)
    val socket1 = sockets[1]
    val hello = assertIs<WsClientMessage.Hello>(socket1.lastOf("hello"))
    assertEquals("TK-2", hello.ticket)

    handshake(socket1)
    val push = assertIs<WsClientMessage.Push>(socket1.lastOf("push"))
    socket1.serverSend(
      WsServerMessage.PushAck(id = push.id, heads = ByteArray(0), durableHeads = ByteArray(0))
    )
    pushDeferred2.await()
    connection.dispose()
  }

  @Test
  fun registerChannelRoutesByDocumentIdAndUnregisters() = runTest {
    val (connection, sockets) = harness()
    val received = mutableListOf<String>()
    val off =
      connection.registerChannel("D1") { message -> received.add(serverMessageTypeOf(message)) }

    connection.sendAttach("D1", null, null)
    runCurrent()
    assertEquals(1, sockets.size)
    val socket = sockets[0]
    assertNull(socket.lastOf("attach"))
    handshake(socket)

    connection.sendAttach("D1", null, null)
    runCurrent()
    assertIs<WsClientMessage.Attach>(socket.lastOf("attach"))

    socket.serverSend(WsServerMessage.AttachAck(documentId = "D1"))
    socket.serverSend(WsServerMessage.AttachAck(documentId = "D2"))
    socket.serverSend(WsServerMessage.Reload(documentId = "D1"))
    runCurrent()
    assertEquals(listOf("attach-ack", "reload"), received)

    off()
    socket.serverSend(WsServerMessage.Reload(documentId = "D1"))
    runCurrent()
    assertEquals(listOf("attach-ack", "reload"), received)
    connection.dispose()
  }

  @Test
  fun pingTwoMissedPongsClosesSocketAndReconnects() = runTest {
    val (connection, sockets) = harness(pingIntervalMs = 30_000, reconnectBaseMs = 1_000)
    connection.registerChannel("D1") {}
    connection.sendAttach("D1", null, null)
    runCurrent()
    val socket0 = sockets[0]
    handshake(socket0)

    advanceTimeBy(30_000)
    runCurrent()
    assertIs<WsClientMessage.Ping>(socket0.lastOf("ping"))

    advanceTimeBy(30_000)
    runCurrent()
    advanceTimeBy(30_000)
    runCurrent()
    assertTrue(socket0.closed.isCompleted)

    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(2, sockets.size)
    connection.dispose()
  }

  @Test
  fun idleTimeoutClosesSocketWithoutSchedulingReconnect() = runTest {
    val (connection, sockets) =
      harness(pingIntervalMs = 120_000, idleTimeoutMs = 60_000, reconnectBaseMs = 1_000)
    val pushDeferred = async { connection.push("D1", byteArrayOf(1)) }
    runCurrent()
    val socket0 = sockets[0]
    handshake(socket0)

    val push = assertIs<WsClientMessage.Push>(socket0.lastOf("push"))
    socket0.serverSend(
      WsServerMessage.PushAck(id = push.id, heads = ByteArray(0), durableHeads = ByteArray(0))
    )
    pushDeferred.await()

    advanceTimeBy(60_000)
    runCurrent()
    assertTrue(socket0.closed.isCompleted)

    advanceTimeBy(120_000)
    runCurrent()
    assertEquals(1, sockets.size)

    val pushDeferred2 = async { connection.push("D1", byteArrayOf(2)) }
    runCurrent()
    assertEquals(2, sockets.size)
    val socket1 = sockets[1]
    handshake(socket1)
    val push2 = assertIs<WsClientMessage.Push>(socket1.lastOf("push"))
    socket1.serverSend(
      WsServerMessage.PushAck(id = push2.id, heads = ByteArray(0), durableHeads = ByteArray(0))
    )
    pushDeferred2.await()
    connection.dispose()
  }

  @Test
  fun detachThenAttachArriveInWireOrder() = runTest {
    val (connection, sockets) = harness()
    connection.registerChannel("D1") {}
    connection.sendAttach("D1", null, null)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)

    val before = socket.sent.size
    connection.sendDetach("D1")
    connection.sendAttach("D1", "5-0", null)
    runCurrent()

    val after = socket.sent.drop(before)
    assertEquals(2, after.size)
    assertIs<WsClientMessage.Detach>(after[0])
    assertIs<WsClientMessage.Attach>(after[1])
    connection.dispose()
  }
}
