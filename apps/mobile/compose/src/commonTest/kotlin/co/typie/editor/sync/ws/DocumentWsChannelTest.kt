package co.typie.editor.sync.ws

import co.typie.editor.sync.RemoteChangesetEvent
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

private const val DOC_ID = "D1"

private data class ChannelHarness(
  val connection: SyncWsConnection,
  val channel: DocumentWsChannel,
  val sockets: MutableList<FakeSyncWsSocket>,
)

private fun TestScope.harness(
  reconnectBaseMs: Long = 1_000,
  idleTimeoutMs: Long = 600_000,
): ChannelHarness {
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
      reconnectBaseMs = reconnectBaseMs,
      idleTimeoutMs = idleTimeoutMs,
    )
  val channel = DocumentWsChannel(connection, DOC_ID, backgroundScope)
  return ChannelHarness(connection, channel, sockets)
}

private fun TestScope.handshake(socket: FakeSyncWsSocket) {
  runCurrent()
  socket.serverSend(WsServerMessage.HelloAck())
  runCurrent()
}

private fun <T> TestScope.collectJob(flow: Flow<T>, sink: MutableList<T>): Job =
  backgroundScope.launch {
    flow.collect { sink.add(it) }
  }

private fun chunk(rowId: String, seq: Int, offset: Int, bytes: ByteArray) =
  WsServerMessage.SnapshotChunk(
    documentId = DOC_ID,
    rowId = rowId,
    seq = seq,
    offset = offset,
    bytes = bytes,
  )

private fun snapshotEnd(
  seq: String,
  heads: ByteArray = ByteArray(0),
  durableHeads: ByteArray = ByteArray(0),
) =
  WsServerMessage.SnapshotEnd(
    documentId = DOC_ID,
    seq = seq,
    heads = heads,
    durableHeads = durableHeads,
  )

private fun changesets(
  seq: String,
  bundles: List<ByteArray>,
  heads: ByteArray = ByteArray(0),
  durableHeads: ByteArray = ByteArray(0),
) =
  WsServerMessage.Changesets(
    documentId = DOC_ID,
    seq = seq,
    bundles = bundles,
    heads = heads,
    durableHeads = durableHeads,
  )

private fun transientDocumentError(code: String) =
  WsServerMessage.WsError(scope = "document", documentId = DOC_ID, code = code, permanent = false)

private fun permanentDocumentError(code: String) =
  WsServerMessage.WsError(scope = "document", documentId = DOC_ID, code = code, permanent = true)

private sealed interface EventSnapshot {
  data class Chunk(val rowId: String, val seq: Int, val offset: Int, val bytes: List<Byte>) :
    EventSnapshot

  data object Restart : EventSnapshot

  data class End(val seq: String, val heads: List<Byte>, val durableHeads: List<Byte>) :
    EventSnapshot

  data class Changesets(
    val seq: String,
    val bundles: List<List<Byte>>,
    val heads: List<Byte>,
    val durableHeads: List<Byte>,
  ) : EventSnapshot

  data object Reload : EventSnapshot

  data class PermanentError(val code: String) : EventSnapshot
}

private fun AttachEvent.snapshot(): EventSnapshot =
  when (this) {
    is AttachEvent.SnapshotChunkEvent -> EventSnapshot.Chunk(rowId, seq, offset, bytes.toList())
    is AttachEvent.SnapshotRestart -> EventSnapshot.Restart
    is AttachEvent.SnapshotEndEvent -> EventSnapshot.End(seq, heads.toList(), durableHeads.toList())
    is AttachEvent.ChangesetsEvent ->
      EventSnapshot.Changesets(
        seq,
        bundles.map { it.toList() },
        heads.toList(),
        durableHeads.toList(),
      )
    is AttachEvent.ReloadEvent -> EventSnapshot.Reload
    is AttachEvent.PermanentErrorEvent -> EventSnapshot.PermanentError(code)
  }

private fun List<AttachEvent>.snapshots(): List<EventSnapshot> = map { it.snapshot() }

private fun chunkS(rowId: String, seq: Int, offset: Int, bytes: ByteArray) =
  EventSnapshot.Chunk(rowId, seq, offset, bytes.toList())

private fun endS(
  seq: String,
  heads: ByteArray = ByteArray(0),
  durableHeads: ByteArray = ByteArray(0),
) = EventSnapshot.End(seq, heads.toList(), durableHeads.toList())

private fun changesetsS(
  seq: String,
  bundles: List<ByteArray>,
  heads: ByteArray = ByteArray(0),
  durableHeads: ByteArray = ByteArray(0),
) =
  EventSnapshot.Changesets(seq, bundles.map { it.toList() }, heads.toList(), durableHeads.toList())

class DocumentWsChannelTest {
  @Test
  fun chunksThenEndArriveInOrderWithCursorTracking() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    assertIs<WsClientMessage.Attach>(socket.lastOf("attach"))

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1, 2)))
    socket.serverSend(chunk("B1", 1, 2, byteArrayOf(3)))
    socket.serverSend(chunk("B2", 2, 0, byteArrayOf(4)))
    socket.serverSend(snapshotEnd("5-0", byteArrayOf(9), byteArrayOf(8)))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1, 2)),
        chunkS("B1", 1, 2, byteArrayOf(3)),
        chunkS("B2", 2, 0, byteArrayOf(4)),
        endS("5-0", byteArrayOf(9), byteArrayOf(8)),
      ),
      events.snapshots(),
    )
  }

  @Test
  fun resumeProbeAcceptedContinuesAccumulation() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), events)
    runCurrent()
    handshake(sockets[0])
    sockets[0].serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    sockets[0].serverSend(chunk("B1", 1, 0, byteArrayOf(1, 2)))
    runCurrent()

    sockets[0].serverClose(1006)
    advanceTimeBy(1_000)
    runCurrent()
    assertEquals(2, sockets.size)
    val socket1 = sockets[1]
    handshake(socket1)

    val resumeAttach = assertIs<WsClientMessage.Attach>(socket1.lastOf("attach"))
    assertEquals(WsSnapshotCursor(rowId = "B1", seq = 1, offset = 2), resumeAttach.snapshotCursor)
    assertNull(resumeAttach.sinceSeq)

    socket1.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket1.serverSend(chunk("B1", 1, 2, byteArrayOf(3)))
    socket1.serverSend(snapshotEnd("5-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1, 2)),
        chunkS("B1", 1, 2, byteArrayOf(3)),
        endS("5-0"),
      ),
      events.snapshots(),
    )
    assertEquals(
      2,
      socket1.sent.count { it is WsClientMessage.Attach } +
        sockets[0].sent.count { it is WsClientMessage.Attach },
    )
  }

  @Test
  fun resumeProbeRejectedEmitsRestartThenNewGeneration() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), events)
    runCurrent()
    handshake(sockets[0])
    sockets[0].serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    sockets[0].serverSend(chunk("B1", 1, 0, byteArrayOf(1, 2)))
    runCurrent()

    sockets[0].serverClose(1006)
    advanceTimeBy(1_000)
    runCurrent()
    val socket1 = sockets[1]
    handshake(socket1)
    assertEquals(
      WsSnapshotCursor(rowId = "B1", seq = 1, offset = 2),
      (socket1.lastOf("attach") as WsClientMessage.Attach).snapshotCursor,
    )

    socket1.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket1.serverSend(chunk("B9", 9, 0, byteArrayOf(7, 7)))
    socket1.serverSend(snapshotEnd("9-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1, 2)),
        EventSnapshot.Restart,
        chunkS("B9", 9, 0, byteArrayOf(7, 7)),
        endS("9-0"),
      ),
      events.snapshots(),
    )
    assertEquals(1, socket1.sent.count { it is WsClientMessage.Attach })
    assertTrue(socket1.sent.none { it is WsClientMessage.Detach })
  }

  @Test
  fun sameRowDiscontinuityTriggersRestartAndFreshReattach() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1, 2)))
    runCurrent()

    socket.serverSend(chunk("B1", 1, 5, byteArrayOf(9)))
    runCurrent()

    assertEquals(
      listOf(chunkS("B1", 1, 0, byteArrayOf(1, 2)), EventSnapshot.Restart),
      events.snapshots(),
    )
    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })
    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })
    val secondAttach = socket.sent.filterIsInstance<WsClientMessage.Attach>()[1]
    assertNull(secondAttach.sinceSeq)
    assertNull(secondAttach.snapshotCursor)

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B2", 2, 0, byteArrayOf(4)))
    socket.serverSend(snapshotEnd("2-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1, 2)),
        EventSnapshot.Restart,
        chunkS("B2", 2, 0, byteArrayOf(4)),
        endS("2-0"),
      ),
      events.snapshots(),
    )
  }

  @Test
  fun reconnectAfterLiveReattachesWithSinceSeqAndFiltersStaleTail() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.events, events)
    runCurrent()
    val socket0 = sockets[0]
    handshake(socket0)
    socket0.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket0.serverSend(snapshotEnd("6-0"))
    socket0.serverSend(changesets("7-0", listOf(byteArrayOf(7))))
    runCurrent()
    assertEquals(
      listOf(endS("6-0"), changesetsS("7-0", listOf(byteArrayOf(7)))),
      events.snapshots(),
    )

    socket0.serverClose(1006)
    advanceTimeBy(1_000)
    runCurrent()
    val socket1 = sockets[1]
    handshake(socket1)
    val resumeAttach = assertIs<WsClientMessage.Attach>(socket1.lastOf("attach"))
    assertEquals("7-0", resumeAttach.sinceSeq)
    assertNull(resumeAttach.snapshotCursor)

    socket1.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket1.serverSend(changesets("5-0", listOf(byteArrayOf(5))))
    runCurrent()
    assertEquals(
      listOf(endS("6-0"), changesetsS("7-0", listOf(byteArrayOf(7)))),
      events.snapshots(),
    )

    socket1.serverSend(changesets("8-0", listOf(byteArrayOf(8))))
    runCurrent()
    assertEquals(
      listOf(
        endS("6-0"),
        changesetsS("7-0", listOf(byteArrayOf(7))),
        changesetsS("8-0", listOf(byteArrayOf(8))),
      ),
      events.snapshots(),
    )
  }

  @Test
  fun emptyBundlesChangesetsStillAdvancesLiveSeqAcrossReconnect() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.events, events)
    runCurrent()
    val socket0 = sockets[0]
    handshake(socket0)
    socket0.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket0.serverSend(snapshotEnd("6-0"))
    socket0.serverSend(changesets("7-0", emptyList()))
    runCurrent()
    assertEquals(
      listOf(endS("6-0"), changesetsS("7-0", emptyList())),
      events.snapshots(),
    )

    socket0.serverClose(1006)
    advanceTimeBy(1_000)
    runCurrent()
    val socket1 = sockets[1]
    handshake(socket1)
    val resumeAttach = assertIs<WsClientMessage.Attach>(socket1.lastOf("attach"))
    assertEquals("7-0", resumeAttach.sinceSeq)
    assertNull(resumeAttach.snapshotCursor)
  }

  @Test
  fun concurrentSubscribersProduceSingleAttachFrame() = runTest {
    val (_, channel, sockets) = harness()
    val loaderEvents = mutableListOf<AttachEvent>()
    val pipelineEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), loaderEvents)
    collectJob(channel.events, pipelineEvents)
    runCurrent()
    assertEquals(1, sockets.size)
    val socket = sockets[0]
    handshake(socket)

    assertEquals(1, socket.sent.count { it is WsClientMessage.Attach })
    assertTrue(socket.sent.none { it is WsClientMessage.Detach })
  }

  @Test
  fun reloadSuspendsSubscribeInsteadOfCompleting() = runTest {
    val (connection, channel, sockets) = harness()
    var reloadCalls = 0
    val transport =
      WsSyncTransport(
        channel,
        connection,
        DOC_ID,
        onReload = { reloadCalls += 1 },
        scope = backgroundScope,
      )
    val received = mutableListOf<RemoteChangesetEvent>()
    val job = collectJob(transport.subscribe(null), received)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    runCurrent()

    socket.serverSend(WsServerMessage.Reload(documentId = DOC_ID))
    runCurrent()

    assertEquals(1, reloadCalls)
    assertTrue(job.isActive)
    assertFalse(job.isCompleted)

    job.cancel()
    runCurrent()
    assertTrue(job.isCompleted)
  }

  @Test
  fun lastSubscriberUnsubscribeSendsDetach() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    val job = collectJob(channel.events, events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    assertEquals(1, socket.sent.count { it is WsClientMessage.Attach })

    job.cancel()
    runCurrent()
    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })
  }

  @Test
  fun lateFreshSubscribeAfterLiveTriggersDetachAndFreshReattach() = runTest {
    val (_, channel, sockets) = harness()
    val firstEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), firstEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    socket.serverSend(snapshotEnd("3-0"))
    runCurrent()
    assertEquals(listOf(chunkS("B1", 1, 0, byteArrayOf(1)), endS("3-0")), firstEvents.snapshots())

    val secondEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), secondEvents)
    runCurrent()

    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })
    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B5", 5, 0, byteArrayOf(9)))
    socket.serverSend(snapshotEnd("4-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1)),
        endS("3-0"),
        EventSnapshot.Restart,
        chunkS("B5", 5, 0, byteArrayOf(9)),
        endS("4-0"),
      ),
      firstEvents.snapshots(),
    )
    assertEquals(
      listOf(EventSnapshot.Restart, chunkS("B5", 5, 0, byteArrayOf(9)), endS("4-0")),
      secondEvents.snapshots(),
    )
  }

  @Test
  fun freshSubscribeOnFirstSubscriberDoesNotReattach() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)

    assertEquals(1, socket.sent.count { it is WsClientMessage.Attach })
    assertTrue(socket.sent.none { it is WsClientMessage.Detach })
  }

  @Test
  fun freshReattachFiltersStaleTailAndPassesNewSeq() = runTest {
    val (_, channel, sockets) = harness()
    val firstEvents = mutableListOf<AttachEvent>()
    collectJob(channel.events, firstEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(snapshotEnd("6-0"))
    socket.serverSend(changesets("7-0", listOf(byteArrayOf(7))))
    runCurrent()

    val secondEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), secondEvents)
    runCurrent()

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    socket.serverSend(snapshotEnd("3-0"))
    runCurrent()

    socket.serverSend(changesets("2-0", listOf(byteArrayOf(2))))
    runCurrent()
    socket.serverSend(changesets("4-0", listOf(byteArrayOf(4))))
    runCurrent()

    assertEquals(
      listOf(
        endS("6-0"),
        changesetsS("7-0", listOf(byteArrayOf(7))),
        EventSnapshot.Restart,
        chunkS("B1", 1, 0, byteArrayOf(1)),
        endS("3-0"),
        changesetsS("4-0", listOf(byteArrayOf(4))),
      ),
      firstEvents.snapshots(),
    )
    assertEquals(
      listOf(
        EventSnapshot.Restart,
        chunkS("B1", 1, 0, byteArrayOf(1)),
        endS("3-0"),
        changesetsS("4-0", listOf(byteArrayOf(4))),
      ),
      secondEvents.snapshots(),
    )
  }

  @Test
  fun streamRegenerationRebasesFilterEvenToLowerSeq() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.events, events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(snapshotEnd("10-0"))
    runCurrent()

    socket.serverSend(WsServerMessage.Reload(documentId = DOC_ID))
    runCurrent()
    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })
    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    socket.serverSend(snapshotEnd("3-0"))
    runCurrent()

    socket.serverSend(changesets("4-0", listOf(byteArrayOf(4))))
    runCurrent()

    assertEquals(
      listOf(
        endS("10-0"),
        EventSnapshot.Reload,
        chunkS("B1", 1, 0, byteArrayOf(1)),
        endS("3-0"),
        changesetsS("4-0", listOf(byteArrayOf(4))),
      ),
      events.snapshots(),
    )
  }

  @Test
  fun secondFreshSubscribeDuringSnapshotConvergesAllToNewGeneration() = runTest {
    val (_, channel, sockets) = harness()
    val firstEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), firstEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    runCurrent()

    val secondEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), secondEvents)
    runCurrent()

    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })
    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B7", 7, 0, byteArrayOf(3)))
    socket.serverSend(snapshotEnd("1-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1)),
        EventSnapshot.Restart,
        chunkS("B7", 7, 0, byteArrayOf(3)),
        endS("1-0"),
      ),
      firstEvents.snapshots(),
    )
    assertEquals(
      listOf(EventSnapshot.Restart, chunkS("B7", 7, 0, byteArrayOf(3)), endS("1-0")),
      secondEvents.snapshots(),
    )
  }

  @Test
  fun restartFenceDiscardsStaleFramesUntilNewAttachAck() = runTest {
    val (_, channel, sockets) = harness()
    val firstEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), firstEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    runCurrent()

    val secondEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), secondEvents)
    runCurrent()
    assertEquals(
      listOf(chunkS("B1", 1, 0, byteArrayOf(1)), EventSnapshot.Restart),
      firstEvents.snapshots(),
    )
    assertEquals(listOf(EventSnapshot.Restart), secondEvents.snapshots())

    socket.serverSend(chunk("B2", 2, 0, byteArrayOf(9)))
    socket.serverSend(snapshotEnd("2-0"))
    socket.serverSend(changesets("9-9", listOf(byteArrayOf(2))))
    runCurrent()
    assertEquals(
      listOf(chunkS("B1", 1, 0, byteArrayOf(1)), EventSnapshot.Restart),
      firstEvents.snapshots(),
    )
    assertEquals(listOf(EventSnapshot.Restart), secondEvents.snapshots())

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B3", 3, 0, byteArrayOf(5)))
    socket.serverSend(snapshotEnd("3-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1)),
        EventSnapshot.Restart,
        chunkS("B3", 3, 0, byteArrayOf(5)),
        endS("3-0"),
      ),
      firstEvents.snapshots(),
    )
    assertEquals(
      listOf(EventSnapshot.Restart, chunkS("B3", 3, 0, byteArrayOf(5)), endS("3-0")),
      secondEvents.snapshots(),
    )
  }

  @Test
  fun pipelineSubscribeStopsEmittingAfterSnapshotRestart() = runTest {
    val (connection, channel, sockets) = harness()
    var reloadCalls = 0
    val transport =
      WsSyncTransport(
        channel,
        connection,
        DOC_ID,
        onReload = { reloadCalls += 1 },
        scope = backgroundScope,
      )
    val pipelineEvents = mutableListOf<RemoteChangesetEvent>()
    val pipelineJob = collectJob(transport.subscribe(null), pipelineEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(snapshotEnd("1-0"))
    socket.serverSend(changesets("2-0", listOf(byteArrayOf(2))))
    runCurrent()
    assertEquals(listOf("2-0"), pipelineEvents.map { it.seq })

    val loaderEvents = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), loaderEvents)
    runCurrent()

    assertEquals(1, reloadCalls)
    assertTrue(pipelineJob.isActive)

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    socket.serverSend(snapshotEnd("1-0"))
    socket.serverSend(changesets("9-0", listOf(byteArrayOf(9))))
    runCurrent()

    assertEquals(listOf("2-0"), pipelineEvents.map { it.seq })

    pipelineJob.cancel()
    runCurrent()
    assertTrue(pipelineJob.isCompleted)
  }

  @Test
  fun transientDocumentErrorDuringSnapshotEmitsRestartThenBackoffReattach() = runTest {
    val (_, channel, sockets) = harness()
    val events = mutableListOf<AttachEvent>()
    collectJob(channel.freshSubscribe(), events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1, 2)))
    runCurrent()

    socket.serverSend(transientDocumentError("internal"))
    runCurrent()

    assertEquals(
      listOf(chunkS("B1", 1, 0, byteArrayOf(1, 2)), EventSnapshot.Restart),
      events.snapshots(),
    )
    assertEquals(1, socket.sent.count { it is WsClientMessage.Attach })

    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })
    val retryAttach = socket.sent.filterIsInstance<WsClientMessage.Attach>()[1]
    assertNull(retryAttach.sinceSeq)
    assertNull(retryAttach.snapshotCursor)
    assertTrue(socket.sent.none { it is WsClientMessage.Detach })

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B2", 2, 0, byteArrayOf(4)))
    socket.serverSend(snapshotEnd("2-0"))
    runCurrent()

    assertEquals(
      listOf(
        chunkS("B1", 1, 0, byteArrayOf(1, 2)),
        EventSnapshot.Restart,
        chunkS("B2", 2, 0, byteArrayOf(4)),
        endS("2-0"),
      ),
      events.snapshots(),
    )
  }

  @Test
  fun permanentFailureThenEvictedChannelReplacementAllowsRetryScheduling() = runTest {
    val (connection, channel, sockets) = harness()
    val firstEvents = mutableListOf<AttachEvent>()
    val job = collectJob(channel.events, firstEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(permanentDocumentError("forbidden"))
    runCurrent()

    assertEquals(listOf(EventSnapshot.PermanentError("forbidden")), firstEvents.snapshots())
    assertEquals(1, socket.sent.count { it is WsClientMessage.Attach })

    job.cancel()
    runCurrent()
    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })

    val freshChannel = DocumentWsChannel(connection, DOC_ID, backgroundScope)
    val secondEvents = mutableListOf<AttachEvent>()
    collectJob(freshChannel.events, secondEvents)
    runCurrent()
    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })

    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(transientDocumentError("internal"))
    runCurrent()

    advanceTimeBy(1_000)
    runCurrent()

    assertEquals(3, socket.sent.count { it is WsClientMessage.Attach })
    assertTrue(secondEvents.snapshots().none { it == EventSnapshot.PermanentError("forbidden") })
  }

  @Test
  fun lastSubscriberDetachStopsFrameRoutingAndRestoresIdleClose() = runTest {
    val (_, channel, sockets) = harness(idleTimeoutMs = 5_000)
    val events = mutableListOf<AttachEvent>()
    val job = collectJob(channel.events, events)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    runCurrent()

    job.cancel()
    runCurrent()
    assertEquals(1, socket.sent.count { it is WsClientMessage.Detach })
    assertFalse(socket.closed.isCompleted)

    val postDetachEvents = mutableListOf<AttachEvent>()
    collectJob(channel.events, postDetachEvents)
    runCurrent()
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(1)))
    socket.serverSend(snapshotEnd("1-0"))
    runCurrent()

    assertEquals(emptyList<EventSnapshot>(), postDetachEvents.snapshots())
    assertEquals(1, socket.sent.count { it is WsClientMessage.Attach })

    advanceTimeBy(5_000)
    runCurrent()
    assertTrue(socket.closed.isCompleted)
  }

  @Test
  fun onEvictedFiresOnceOnLastDetachAndReplacementChannelAttachesIndependently() = runTest {
    val (connection, _, sockets) = harness()
    var evictedCount = 0
    val trackedChannel =
      DocumentWsChannel(connection, DOC_ID, backgroundScope) { evictedCount += 1 }
    val trackedEvents = mutableListOf<AttachEvent>()
    val trackedJob = collectJob(trackedChannel.events, trackedEvents)
    runCurrent()
    val socket = sockets[0]
    handshake(socket)
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    runCurrent()
    assertEquals(0, evictedCount)

    trackedJob.cancel()
    runCurrent()
    assertEquals(1, evictedCount)

    val replacementChannel = DocumentWsChannel(connection, DOC_ID, backgroundScope)
    val replacementEvents = mutableListOf<AttachEvent>()
    collectJob(replacementChannel.events, replacementEvents)
    runCurrent()

    assertEquals(2, socket.sent.count { it is WsClientMessage.Attach })
    socket.serverSend(WsServerMessage.AttachAck(documentId = DOC_ID))
    socket.serverSend(chunk("B1", 1, 0, byteArrayOf(9)))
    socket.serverSend(snapshotEnd("1-0"))
    runCurrent()

    assertEquals(
      listOf(chunkS("B1", 1, 0, byteArrayOf(9)), endS("1-0")),
      replacementEvents.snapshots(),
    )
    assertEquals(emptyList<EventSnapshot>(), trackedEvents.snapshots())
    assertEquals(1, evictedCount)
  }
}
