package co.typie.editor.sync

import co.typie.editor.sync.ws.SyncWsException
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

private class FakeTransport : SyncTransport {
  val pushCalls = mutableListOf<ByteArray>()
  val pullCalls = mutableListOf<String?>()
  val subscribeCalls = mutableListOf<String?>()
  var pullResult: PullResult =
    PullResult(
      changesets = emptyList(),
      seq = "",
      heads = enc(),
      durableHeads = enc(),
      needsReload = false,
    )
  val subscriptionEvents = Channel<RemoteChangesetEvent>(Channel.UNLIMITED)
  var failSubscriptionAfterFirstEvent = false
  var subscribeError: Throwable? = null

  override suspend fun push(changesets: ByteArray): PushResult {
    pushCalls.add(changesets)
    return PushResult(heads = enc(), durableHeads = enc())
  }

  override suspend fun pull(sinceSeq: String?): PullResult {
    pullCalls.add(sinceSeq)
    return pullResult
  }

  override fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent> {
    subscribeCalls.add(sinceSeq)
    return flow {
      subscribeError?.let { throw it }
      for (event in subscriptionEvents) {
        emit(event)
        if (failSubscriptionAfterFirstEvent) throw RuntimeException("ws closed")
      }
    }
  }
}

private class RecordingHeadsSink : SyncHeadsSink {
  val confirmed = mutableListOf<ByteArray>()
  val durable = mutableListOf<ByteArray>()

  override fun setConfirmedHeads(heads: ByteArray) {
    confirmed.add(heads)
  }

  override fun setDurableHeads(heads: ByteArray) {
    durable.add(heads)
  }
}

private data class Harness(
  val editor: FakeSyncEditor,
  val transport: FakeTransport,
  val sink: RecordingHeadsSink,
  val pipeline: RemoteChangesetPipeline,
)

class RemoteChangesetPipelineTest {
  private fun TestScope.harness(
    initialSeq: String = "",
    onNeedsReload: suspend () -> Unit = {},
  ): Harness {
    val editor = FakeSyncEditor()
    val transport = FakeTransport()
    val sink = RecordingHeadsSink()
    val pipeline =
      RemoteChangesetPipeline(
        editor = editor,
        headsSink = sink,
        transport = transport,
        initialSeq = initialSeq,
        scope = CoroutineScope(coroutineContext),
        onNeedsReload = onNeedsReload,
      )
    return Harness(editor, transport, sink, pipeline)
  }

  @Test
  fun subscriptionEventsAreAppliedDirectlyAndPropagateHeads() = runTest {
    val (editor, transport, sink, pipeline) = harness(initialSeq = "s1")
    pipeline.start()
    runCurrent()
    assertEquals(listOf<String?>("s1"), transport.subscribeCalls)

    transport.subscriptionEvents.send(
      RemoteChangesetEvent(
        changesets = listOf(enc(7)),
        seq = "s2",
        heads = enc(7),
        durableHeads = enc(3),
      )
    )
    runCurrent()
    assertTrue(7 in editor.known)
    assertContentEquals(enc(7), sink.confirmed.last())
    assertContentEquals(enc(3), sink.durable.last())
    pipeline.stop()
  }

  @Test
  fun reconnectUsesLatestSeq() = runTest {
    val (_, transport, _, pipeline) = harness(initialSeq = "s1")
    transport.failSubscriptionAfterFirstEvent = true
    pipeline.start()
    runCurrent()

    transport.subscriptionEvents.send(
      RemoteChangesetEvent(
        changesets = emptyList(),
        seq = "s5",
        heads = enc(),
        durableHeads = enc(),
      )
    )
    runCurrent()
    advanceTimeBy(1000)
    runCurrent()

    assertEquals(listOf<String?>("s1", "s5"), transport.subscribeCalls)
    pipeline.stop()
  }

  @Test
  fun pollingPullsOnInterval() = runTest {
    val (_, transport, _, pipeline) = harness(initialSeq = "s1")
    pipeline.start()
    runCurrent()
    assertEquals(0, transport.pullCalls.size)

    advanceTimeBy(10_000)
    runCurrent()
    assertEquals(listOf<String?>("s1"), transport.pullCalls)
    pipeline.stop()
  }

  @Test
  fun needsReloadStopsPipelineAndInvokesCallback() = runTest {
    var reloaded = false
    val (_, transport, _, pipeline) =
      harness(initialSeq = "s1", onNeedsReload = { reloaded = true })
    transport.pullResult =
      PullResult(
        changesets = emptyList(),
        seq = "",
        heads = enc(),
        durableHeads = enc(),
        needsReload = true,
      )
    pipeline.start()
    runCurrent()

    pipeline.refetchFromServer()
    assertTrue(reloaded)

    advanceTimeBy(30_000)
    runCurrent()
    assertEquals(1, transport.pullCalls.size)
    pipeline.stop()
  }

  @Test
  fun emptySeqIsSentAsNull() = runTest {
    val (_, transport, _, pipeline) = harness(initialSeq = "")
    pipeline.start()
    runCurrent()
    assertEquals(listOf<String?>(null), transport.subscribeCalls)
    pipeline.stop()
  }

  @Test
  fun pullResultsPropagateHeads() = runTest {
    val (_, transport, sink, pipeline) = harness(initialSeq = "s1")
    transport.pullResult =
      PullResult(
        changesets = emptyList(),
        seq = "s3",
        heads = enc(9),
        durableHeads = enc(4),
        needsReload = false,
      )
    pipeline.refetchFromServer()
    assertContentEquals(enc(9), sink.confirmed.last())
    assertContentEquals(enc(4), sink.durable.last())
    pipeline.stop()
  }

  @Test
  fun startIsIdempotent() = runTest {
    val (_, transport, _, pipeline) = harness(initialSeq = "s1")
    pipeline.start()
    pipeline.start()
    runCurrent()
    assertEquals(1, transport.subscribeCalls.size)
    pipeline.stop()
  }

  @Test
  fun needsReloadFromPollingPathStillRunsCallback() = runTest {
    var reloaded = false
    val (_, transport, _, pipeline) =
      harness(
        initialSeq = "s1",
        onNeedsReload = {
          delay(50)
          reloaded = true
        },
      )
    transport.pullResult =
      PullResult(
        changesets = emptyList(),
        seq = "",
        heads = enc(),
        durableHeads = enc(),
        needsReload = true,
      )
    pipeline.start()
    runCurrent()

    advanceTimeBy(10_000)
    runCurrent()
    advanceTimeBy(50)
    runCurrent()

    assertTrue(reloaded)
    assertEquals(1, transport.pullCalls.size)

    advanceTimeBy(30_000)
    runCurrent()
    assertEquals(1, transport.pullCalls.size)
  }

  @Test
  fun permanentSubscriptionErrorStopsReconnectLoop() = runTest {
    val (_, transport, _, pipeline) = harness(initialSeq = "s1")
    transport.subscribeError = SyncWsException("forbidden", true)
    pipeline.start()
    runCurrent()
    advanceTimeBy(60_000)
    runCurrent()
    assertEquals(1, transport.subscribeCalls.size)
    pipeline.stop()
  }
}
