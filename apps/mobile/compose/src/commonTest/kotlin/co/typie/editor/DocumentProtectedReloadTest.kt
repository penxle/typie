package co.typie.editor

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
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.emptyFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class DocumentProtectedReloadTest {
  private class FakeTransport(private val pushFn: suspend (ByteArray) -> PushResult) :
    SyncTransport {
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
    val session: DocumentEditingSession,
    val engine: SyncEngine,
    val syncEditor: FakeSyncEditor,
  )

  private fun TestScope.harness(
    store: FakeDeltaStore = FakeDeltaStore(),
    pushFn: suspend (ByteArray) -> PushResult = { PushResult(heads = enc(), durableHeads = enc()) },
  ): Harness {
    val syncEditor = FakeSyncEditor()
    val scope = CoroutineScope(coroutineContext)
    val transport = FakeTransport(pushFn)
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
        editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)),
        engine = engine,
        pipeline = pipeline,
        scope = scope,
      )
    runCurrent()
    return Harness(session = session, engine = engine, syncEditor = syncEditor)
  }

  private fun failingStore(onPut: (suspend () -> Unit)? = null): FakeDeltaStore =
    FakeDeltaStore().apply {
      this.onPut = {
        onPut?.invoke()
        throw IllegalStateException("disk unavailable")
      }
    }

  private suspend fun failingPush(changesets: ByteArray): PushResult {
    throw IllegalStateException("network unavailable")
  }

  @Test
  fun protectedFrontierReplacesWithoutFailureDecision() = runTest {
    val (session) = harness()
    var decisions = 0

    val result =
      runProtectedDocumentReload(
        session = session,
        finalizeInput = {},
        resolveFailure = {
          decisions += 1
          DocumentReloadFailureDecision.Retry
        },
        replaceIfCurrent = {
          it.stop()
          true
        },
      )

    assertEquals(DocumentProtectedReloadResult.Replaced, result)
    assertEquals(0, decisions)
  }

  @Test
  fun retryKeepsAdmissionClosedAndSharesTheInFlightCheckpoint() = runTest {
    var putCalls = 0
    val retryStarted = CompletableDeferred<Unit>()
    val releaseRetry = CompletableDeferred<Unit>()
    val store = failingStore {
      putCalls += 1
      if (putCalls >= 3) {
        retryStarted.complete(Unit)
        releaseRetry.await()
      }
    }
    val (session, _, syncEditor) = harness(store = store, pushFn = ::failingPush)
    syncEditor.known.add(1)
    var decisions = 0
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          resolveFailure = {
            decisions += 1
            if (decisions == 1) DocumentReloadFailureDecision.Retry else awaitCancellation()
          },
          replaceIfCurrent = { false },
        )
      }
    retryStarted.await()

    val observerStop = session.beginStop()
    val observer = async { observerStop.retryCheckpoint() }
    runCurrent()

    assertEquals(3, putCalls)
    assertNull(session.submit { _, context -> async(context) {} })

    policy.cancelAndJoin()
    assertNull(session.submit { _, context -> async(context) {} })
    releaseRetry.complete(Unit)
    observer.cancelAndJoin()
    observerStop.cancel()
    assertNotNull(session.submit { _, context -> async(context) {} }).join()
    session.stop()
  }

  @Test
  fun immediateRetryFailureWaitsTheCompleteProductWindow() = runTest {
    val (session, _, syncEditor) = harness(store = failingStore(), pushFn = ::failingPush)
    syncEditor.known.add(1)
    var decisions = 0
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          resolveFailure = {
            decisions += 1
            if (decisions == 1) DocumentReloadFailureDecision.Retry else awaitCancellation()
          },
          replaceIfCurrent = { false },
        )
      }
    runCurrent()

    advanceTimeBy(2_999)
    runCurrent()
    assertEquals(1, decisions)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(2, decisions)

    policy.cancelAndJoin()
    session.stop()
  }

  @Test
  fun exactProtectionCancelsTheVisibleDecisionAndReplaces() = runTest {
    val (session, engine, syncEditor) = harness(store = failingStore(), pushFn = ::failingPush)
    syncEditor.known.add(1)
    val dialogStarted = CompletableDeferred<Unit>()
    var dialogCancelled = false
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          resolveFailure = {
            dialogStarted.complete(Unit)
            try {
              awaitCancellation()
            } finally {
              dialogCancelled = true
            }
          },
          replaceIfCurrent = {
            it.stop()
            true
          },
        )
      }
    dialogStarted.await()

    engine.setConfirmedHeads(enc(1))
    runCurrent()

    assertEquals(DocumentProtectedReloadResult.Replaced, policy.await())
    assertTrue(dialogCancelled)
  }

  @Test
  fun partialProtectionRechecksWithoutExtendingTheRecoveryWindow() = runTest {
    val (session, engine, syncEditor) = harness(store = failingStore(), pushFn = ::failingPush)
    syncEditor.known.addAll(listOf(1, 2, 3))
    var decisions = 0
    var replacements = 0
    val firstDialog = CompletableDeferred<Unit>()
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          resolveFailure = {
            decisions += 1
            firstDialog.complete(Unit)
            awaitCancellation()
          },
          replaceIfCurrent = {
            replacements += 1
            false
          },
        )
      }
    firstDialog.await()

    engine.setConfirmedHeads(enc(1))
    runCurrent()
    advanceTimeBy(1_000)
    engine.setConfirmedHeads(enc(2))
    runCurrent()
    advanceTimeBy(1_999)
    runCurrent()

    assertEquals(1, decisions)
    assertEquals(0, replacements)

    advanceTimeBy(1)
    runCurrent()
    assertEquals(2, decisions)
    assertEquals(0, replacements)

    policy.cancelAndJoin()
    session.stop()
  }

  @Test
  fun discardReplacesTheExactUnprotectedSession() = runTest {
    val (session, _, syncEditor) = harness(store = failingStore(), pushFn = ::failingPush)
    syncEditor.known.add(1)

    val result =
      runProtectedDocumentReload(
        session = session,
        finalizeInput = {},
        resolveFailure = { DocumentReloadFailureDecision.Discard },
        replaceIfCurrent = {
          it.stop()
          true
        },
      )

    assertEquals(DocumentProtectedReloadResult.Replaced, result)
  }

  @Test
  fun cancellationReleasesOnlyTheReloadStopHandle() = runTest {
    val (session, _, syncEditor) = harness(store = failingStore(), pushFn = ::failingPush)
    syncEditor.known.add(1)
    val dialogStarted = CompletableDeferred<Unit>()
    var dialogCancelled = false
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          resolveFailure = {
            dialogStarted.complete(Unit)
            try {
              awaitCancellation()
            } finally {
              dialogCancelled = true
            }
          },
          replaceIfCurrent = { false },
        )
      }
    dialogStarted.await()
    val otherStop = session.beginStop()

    policy.cancelAndJoin()

    assertTrue(dialogCancelled)
    assertNull(session.submit { _, context -> async(context) {} })
    otherStop.cancel()
    assertNotNull(session.submit { _, context -> async(context) {} }).join()
    session.stop()
  }

  @Test
  fun feedbackCleanupFailureDoesNotFailReloadAndStillReleasesTheStop() = runTest {
    val (session) = harness()

    assertEquals(
      DocumentProtectedReloadResult.NotCurrent,
      runProtectedDocumentReload(
        session = session,
        finalizeInput = {},
        hideDelayedFeedback = { error("feedback cleanup failed") },
        resolveFailure = { error("protected frontier must not ask") },
        replaceIfCurrent = { false },
      ),
    )

    assertNotNull(session.submit { _, context -> async(context) {} }).join()
    session.stop()
  }

  @Test
  fun delayedFeedbackFailureDoesNotFailReload() = runTest {
    val putStarted = CompletableDeferred<Unit>()
    val releasePut = CompletableDeferred<Unit>()
    val store = failingStore {
      putStarted.complete(Unit)
      releasePut.await()
    }
    val (session, _, syncEditor) = harness(store = store, pushFn = ::failingPush)
    syncEditor.known.add(1)
    var feedbackAttempts = 0
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          showDelayedFeedback = {
            feedbackAttempts += 1
            error("feedback display failed")
          },
          resolveFailure = { DocumentReloadFailureDecision.Discard },
          replaceIfCurrent = { false },
          delayedFeedbackMillis = 10,
          checkpointWatchdogMillis = 20,
        )
      }
    putStarted.await()

    advanceTimeBy(20)
    runCurrent()

    assertEquals(DocumentProtectedReloadResult.NotCurrent, policy.await())
    assertEquals(1, feedbackAttempts)
    releasePut.complete(Unit)
    runCurrent()
    assertNotNull(session.submit { _, context -> async(context) {} }).join()
    session.stop()
  }

  @Test
  fun failedIdentityClaimReleasesAdmissionWithoutClaimingAnotherGeneration() = runTest {
    val (session) = harness()
    var claims = 0

    val result =
      runProtectedDocumentReload(
        session = session,
        finalizeInput = {},
        resolveFailure = { error("protected frontier must not ask") },
        replaceIfCurrent = {
          claims += 1
          false
        },
      )

    assertEquals(DocumentProtectedReloadResult.NotCurrent, result)
    assertEquals(1, claims)
    assertNotNull(session.submit { _, context -> async(context) {} }).join()
    session.stop()
  }

  @Test
  fun stopAcquisitionCallbackRunsBeforeThePolicySuspends() = runTest {
    val putStarted = CompletableDeferred<Unit>()
    val releasePut = CompletableDeferred<Unit>()
    val releasePush = CompletableDeferred<Unit>()
    val store = failingStore {
      putStarted.complete(Unit)
      releasePut.await()
    }
    val (session, _, syncEditor) =
      harness(
        store = store,
        pushFn = {
          releasePush.await()
          failingPush(it)
        },
      )
    syncEditor.known.add(1)
    val events = mutableListOf<String>()

    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = { events += "finalize" },
          onStopAcquired = { events += "acquired" },
          resolveFailure = { awaitCancellation() },
          replaceIfCurrent = { false },
        )
      }
    putStarted.await()

    assertEquals(listOf("finalize", "acquired"), events)
    assertFalse(policy.isCompleted)

    policy.cancelAndJoin()
    releasePut.complete(Unit)
    releasePush.complete(Unit)
    runCurrent()
    session.stop()
  }

  @Test
  fun protectionAdvanceDuringAFailedAttemptIsObservedFromThePreAttemptCursor() = runTest {
    var putCalls = 0
    val firstPutStarted = CompletableDeferred<Unit>()
    val releaseFirstPut = CompletableDeferred<Unit>()
    val retryStarted = CompletableDeferred<Unit>()
    val store = failingStore {
      putCalls += 1
      when (putCalls) {
        1 -> {
          firstPutStarted.complete(Unit)
          releaseFirstPut.await()
        }
        3 -> retryStarted.complete(Unit)
      }
    }
    val (session, engine, syncEditor) = harness(store = store, pushFn = ::failingPush)
    syncEditor.known.addAll(listOf(1, 2))
    val policy =
      async(start = CoroutineStart.UNDISPATCHED) {
        runProtectedDocumentReload(
          session = session,
          finalizeInput = {},
          resolveFailure = { awaitCancellation() },
          replaceIfCurrent = { false },
        )
      }
    firstPutStarted.await()

    engine.setConfirmedHeads(enc(1))
    releaseFirstPut.complete(Unit)
    retryStarted.await()

    assertTrue(putCalls >= 3)
    assertNull(session.submit { _, context -> async(context) {} })

    policy.cancelAndJoin()
    session.stop()
  }
}
