package co.typie.editor

import androidx.compose.runtime.snapshots.Snapshot
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.editor.sync.FakeDeltaStore
import co.typie.editor.sync.FakeSyncEditor
import co.typie.editor.sync.PullResult
import co.typie.editor.sync.PushResult
import co.typie.editor.sync.RemoteChangesetEvent
import co.typie.editor.sync.RemoteChangesetPipeline
import co.typie.editor.sync.SyncEngine
import co.typie.editor.sync.SyncTransport
import co.typie.editor.sync.enc
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class DocumentEditingSessionTest {
  private class FakeTransport(
    private val pushFn: suspend (ByteArray) -> PushResult = {
      PushResult(heads = enc(), durableHeads = enc())
    }
  ) : SyncTransport {
    override suspend fun push(changesets: ByteArray): PushResult = pushFn(changesets)

    override suspend fun pull(sinceSeq: String?): PullResult =
      PullResult(
        changesets = emptyList(),
        seq = "",
        heads = enc(),
        durableHeads = enc(),
        needsReload = false,
      )

    override fun subscribe(sinceSeq: String?): Flow<RemoteChangesetEvent> = emptyFlow()
  }

  private data class Harness(
    val editor: Editor,
    val session: DocumentEditingSession,
    val store: FakeDeltaStore,
  )

  private fun TestScope.harness(
    store: FakeDeltaStore = FakeDeltaStore(),
    syncEditor: FakeSyncEditor = FakeSyncEditor(),
    editor: Editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)),
    pushFn: suspend (ByteArray) -> PushResult = { PushResult(heads = enc(), durableHeads = enc()) },
  ): Harness {
    val transport = FakeTransport(pushFn)
    val scope = CoroutineScope(coroutineContext)
    val engine =
      SyncEngine(
        editor = syncEditor,
        documentId = "doc",
        initialServerHeads = enc(),
        initialDurableHeads = enc(),
        store = store,
        pushFn = transport::push,
        scope = scope,
        now = { 0L },
      )
    val pipeline =
      RemoteChangesetPipeline(
        editor = syncEditor,
        headsSink = engine,
        transport = transport,
        initialSeq = "",
        scope = scope,
        onNeedsReload = {},
      )
    val session =
      DocumentEditingSession(
        documentId = "doc",
        editor = editor,
        engine = engine,
        pipeline = pipeline,
        scope = scope,
      )
    return Harness(editor = editor, session = session, store = store)
  }

  @Test
  fun closeSucceedsThroughExactLiveAckWhenLocalCaptureFails() = runTest {
    val store = FakeDeltaStore()
    store.onPut = { throw IllegalStateException("disk unavailable") }
    val syncEditor = FakeSyncEditor()
    val (_, session) =
      harness(
        store = store,
        syncEditor = syncEditor,
        pushFn = { PushResult(heads = enc(1), durableHeads = enc()) },
      )
    runCurrent()
    syncEditor.known.add(1)

    val result = session.beginStop().awaitCheckpoint()

    assertEquals(EditingCheckpointResult.Protected, result)
  }

  @Test
  fun editFailureRemainsTerminalWhenLiveAckProtectsCommittedState() = runTest {
    val store = FakeDeltaStore()
    store.onPut = { throw IllegalStateException("disk unavailable") }
    val failure = IllegalStateException("edit failed")
    val syncEditor = FakeSyncEditor()
    val (_, session) =
      harness(
        store = store,
        syncEditor = syncEditor,
        pushFn = { PushResult(heads = enc(1), durableHeads = enc()) },
      )
    runCurrent()
    syncEditor.known.add(1)
    session.submit { _, _ -> CompletableDeferred<Unit>().apply { completeExceptionally(failure) } }

    val result = session.beginStop().awaitCheckpoint()

    assertTrue(result is EditingCheckpointResult.EditFailed)
    assertEquals(failure, result.cause)
  }

  @Test
  fun documentRevisionSchedulesLocalCaptureAfterSessionStart() = runTest {
    val syncEditor = FakeSyncEditor()
    val editor =
      Editor(
        FakeFfiEditor(
          onTick = {
            syncEditor.known.add(1)
            listOf(EditorEvent.StateChanged(listOf(StateField.Doc)))
          }
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    val (_, session, store) = harness(editor = editor, syncEditor = syncEditor)
    session.start()
    runCurrent()

    editor.await { enqueue(Message.System(SystemEvent.Initialize)) }
    assertEquals(1L, editor.state.documentRevision)
    Snapshot.sendApplyNotifications()
    runCurrent()

    assertEquals(listOf("1"), store.load("doc").map { it.id })
    session.stop()
  }

  @Test
  fun stoppedSessionCannotStart() = runTest {
    val (_, session) = harness()
    session.stop()

    assertFailsWith<IllegalStateException> { session.start() }
  }

  @Test
  fun closeWaitsForSubmissionAcceptedBeforeItsCoroutineCompletes() = runTest {
    val (editor, session) = harness()
    val gate = CompletableDeferred<Unit>()
    var started = false

    val accepted = session.submit { sessionEditor, context ->
      assertSame(editor, sessionEditor)
      async(context = context) {
        started = true
        gate.await()
        sessionEditor.await { enqueue(Message.System(SystemEvent.Initialize)) }
      }
    }
    assertNotNull(accepted)
    assertFalse(started)

    val close = session.beginStop()
    val result = async(start = CoroutineStart.UNDISPATCHED) { close.awaitCheckpoint() }

    assertNull(session.submit { _, context -> async(context) {} })
    assertFalse(result.isCompleted)

    testScheduler.runCurrent()
    assertTrue(started)
    assertFalse(result.isCompleted)

    gate.complete(Unit)
    advanceUntilIdle()

    assertEquals(EditingCheckpointResult.Protected, result.await())
  }

  @Test
  fun closeCapturesFinalAcceptedEditInPendingStore() = runTest {
    val syncEditor = FakeSyncEditor()
    val editor =
      Editor(
        FakeFfiEditor(
          onTick = {
            syncEditor.known.add(1)
            listOf(EditorEvent.StateChanged(listOf(StateField.Doc)))
          }
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    val (_, session, store) = harness(syncEditor = syncEditor, editor = editor)
    val gate = CompletableDeferred<Unit>()
    val accepted = session.submit { sessionEditor, context ->
      async(context = context) {
        gate.await()
        sessionEditor.await { enqueue(Message.System(SystemEvent.Initialize)) }
      }
    }
    assertNotNull(accepted)

    val close = session.beginStop()
    val result = async(start = CoroutineStart.UNDISPATCHED) { close.awaitCheckpoint() }
    runCurrent()
    assertFalse(result.isCompleted)

    gate.complete(Unit)
    advanceUntilIdle()

    assertEquals(EditingCheckpointResult.Protected, result.await())
    assertEquals(listOf("1"), store.load("doc").map { it.id })
  }

  @Test
  fun staleCloseCannotReopenANewerClose() = runTest {
    val (_, session) = harness()
    val first = session.beginStop()
    first.cancel()
    val second = session.beginStop()

    first.cancel()

    assertNull(session.submit { _, context -> async(context) {} })

    second.cancel()
    assertNotNull(session.submit { _, context -> async(context) {} })
  }

  @Test
  fun stoppingSessionCompletesPendingCloseAsStopped() = runTest {
    val (editor, session) = harness()
    val gate = CompletableDeferred<Unit>()
    session.submit { sessionEditor, context ->
      async(context) {
        gate.await()
        sessionEditor.await { enqueue(Message.System(SystemEvent.Initialize)) }
      }
    }
    val close = session.beginStop()
    val result = async(start = CoroutineStart.UNDISPATCHED) { close.awaitCheckpoint() }

    session.stop()
    testScheduler.runCurrent()
    val completedWhenStopped = result.isCompleted

    gate.complete(Unit)
    advanceUntilIdle()

    assertTrue(completedWhenStopped)
    assertEquals(EditingCheckpointResult.SessionStopped, result.await())
  }

  @Test
  fun cancelledStopCannotReuseCompletedCheckpointResult() = runTest {
    var puts = 0
    val store = FakeDeltaStore()
    val syncEditor = FakeSyncEditor()
    val (_, session) = harness(store = store, syncEditor = syncEditor)
    runCurrent()
    syncEditor.known.add(1)
    store.onPut = {
      puts += 1
      throw IllegalStateException("disk unavailable")
    }
    val close = session.beginStop()
    val failure = close.awaitCheckpoint()
    close.cancel()

    assertTrue(failure is EditingCheckpointResult.ProtectionFailed)
    assertEquals(EditingCheckpointResult.StopCancelled, close.awaitCheckpoint())
    assertEquals(EditingCheckpointResult.StopCancelled, close.retryCheckpoint())
    advanceUntilIdle()
    assertEquals(2, puts)
    assertNotNull(session.submit { _, context -> async(context) {} })
  }

  @Test
  fun editFailureStillCapturesCommittedEditorState() = runTest {
    val store = FakeDeltaStore()
    val syncEditor = FakeSyncEditor()
    val (_, session) = harness(store = store, syncEditor = syncEditor)
    runCurrent()
    syncEditor.known.add(1)
    val failure = IllegalStateException("edit failed")

    session.submit { _, _ -> CompletableDeferred<Unit>().apply { completeExceptionally(failure) } }

    val result = session.beginStop().awaitCheckpoint()

    assertTrue(result is EditingCheckpointResult.EditFailed)
    assertEquals(failure, result.cause)
    assertEquals(listOf("1"), store.load("doc").map { it.id })
  }

  @Test
  fun repeatedAwaitsShareOneCaptureAttempt() = runTest {
    var puts = 0
    val store = FakeDeltaStore()
    val syncEditor = FakeSyncEditor()
    val (_, session) = harness(store = store, syncEditor = syncEditor)
    runCurrent()
    syncEditor.known.add(1)
    store.onPut = { puts += 1 }
    val close = session.beginStop()

    val first = async { close.awaitCheckpoint() }
    val second = async { close.awaitCheckpoint() }
    advanceUntilIdle()

    assertEquals(EditingCheckpointResult.Protected, first.await())
    assertEquals(EditingCheckpointResult.Protected, second.await())
    assertEquals(1, puts)
  }

  @Test
  fun cancellingAwaiterDoesNotCancelSessionOwnedCapture() = runTest {
    val captureGate = CompletableDeferred<Unit>()
    val store = FakeDeltaStore()
    val syncEditor = FakeSyncEditor()
    val (_, session) = harness(store = store, syncEditor = syncEditor)
    runCurrent()
    syncEditor.known.add(1)
    store.onLoad = {
      captureGate.await()
      emptyList()
    }
    val close = session.beginStop()
    val first = async { close.awaitCheckpoint() }

    testScheduler.runCurrent()
    assertFalse(first.isCompleted)

    first.cancel()
    testScheduler.runCurrent()
    assertTrue(first.isCancelled)

    captureGate.complete(Unit)
    advanceUntilIdle()

    assertEquals(EditingCheckpointResult.Protected, close.awaitCheckpoint())
  }

  @Test
  fun cancellingOneOfTwoStopHandlesKeepsAdmissionClosed() = runTest {
    val (_, session) = harness()
    val first = session.beginStop()
    val second = session.beginStop()

    first.cancel()
    assertNull(session.submit { _, _ -> CompletableDeferred(Unit) })

    second.cancel()
    assertNotNull(session.submit { _, _ -> CompletableDeferred(Unit) })
  }

  @Test
  fun cancelledStopHandleCannotObserveSharedProtectedResult() = runTest {
    val (_, session) = harness()
    val editGate = CompletableDeferred<Unit>()
    assertNotNull(session.submit { _, context -> async(context) { editGate.await() } })
    val cancelled = session.beginStop()
    val remaining = session.beginStop()
    val cancelledResult = async { cancelled.awaitCheckpoint() }

    cancelled.cancel()

    assertEquals(EditingCheckpointResult.StopCancelled, cancelledResult.await())
    editGate.complete(Unit)
    advanceUntilIdle()
    assertEquals(EditingCheckpointResult.Protected, remaining.awaitCheckpoint())
    remaining.cancel()
  }

  @Test
  fun concurrentRetriesShareOneAttemptAndKeepTheSameFrontier() = runTest {
    var loads = 0
    val store = FakeDeltaStore()
    store.onLoad = {
      loads += 1
      emptyList()
    }
    store.onPut = { throw IllegalStateException("disk unavailable") }
    val syncEditor = FakeSyncEditor()
    val (_, session) =
      harness(
        store = store,
        syncEditor = syncEditor,
        pushFn = { throw IllegalStateException("network unavailable") },
      )
    runCurrent()
    syncEditor.known.add(1)
    val stop = session.beginStop()
    assertTrue(stop.awaitCheckpoint() is EditingCheckpointResult.ProtectionFailed)
    val loadsBeforeRetry = loads
    val retryGate = CompletableDeferred<Unit>()
    store.onLoad = {
      loads += 1
      retryGate.await()
      emptyList()
    }

    val first = async { stop.retryCheckpoint() }
    runCurrent()
    val second = async { stop.retryCheckpoint() }
    runCurrent()

    assertEquals(loadsBeforeRetry + 1, loads)
    assertNull(session.submit { _, _ -> CompletableDeferred(Unit) })

    retryGate.complete(Unit)
    advanceUntilIdle()

    assertTrue(first.await() is EditingCheckpointResult.ProtectionFailed)
    assertTrue(second.await() is EditingCheckpointResult.ProtectionFailed)
    assertEquals(loadsBeforeRetry + 2, loads)
    stop.cancel()
  }

  @Test
  fun finalReleaseCannotReopenAdmissionBehindANewerStop() = runTest {
    val (editor, session) = harness()

    repeat(100) {
      val first = session.beginStop()
      val raceGate = CompletableDeferred<Unit>()
      val release =
        async(Dispatchers.Default) {
          raceGate.await()
          first.cancel()
        }
      val next =
        async(Dispatchers.Default) {
          raceGate.await()
          session.beginStop()
        }

      raceGate.complete(Unit)
      val second = next.await()
      release.await()

      assertNull(editor.trackLocalEdit { CompletableDeferred(Unit) })
      second.cancel()
      assertNotNull(session.submit { _, _ -> CompletableDeferred(Unit) })
    }
  }
}
