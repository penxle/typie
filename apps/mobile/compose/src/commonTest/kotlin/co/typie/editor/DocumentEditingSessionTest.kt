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

    val result = session.beginClose().awaitCheckpoint()

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

    val result = session.beginClose().awaitCheckpoint()

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

    val close = session.beginClose()
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

    val close = session.beginClose()
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
    val first = session.beginClose()
    first.cancel()
    val second = session.beginClose()

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
    val close = session.beginClose()
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
  fun cancelledCloseKeepsCompletedCheckpointResult() = runTest {
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
    val close = session.beginClose()
    val failure = close.awaitCheckpoint()
    close.cancel()

    assertEquals(failure, close.awaitCheckpoint())
    assertTrue(failure is EditingCheckpointResult.ProtectionFailed)
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

    val result = session.beginClose().awaitCheckpoint()

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
    val close = session.beginClose()

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
    val close = session.beginClose()
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
}
