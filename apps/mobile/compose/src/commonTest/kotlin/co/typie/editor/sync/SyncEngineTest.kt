package co.typie.editor.sync

import co.typie.editor.sync.ws.SyncWsException
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.async
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

class SyncEngineTest {
  private var clock = 0L

  private fun TestScope.engine(
    editor: SyncEditor,
    store: FakeDeltaStore,
    initialServerHeads: ByteArray = enc(),
    initialDurableHeads: ByteArray = enc(),
    isPermanent: (Throwable) -> Boolean = { false },
    onEvent: (SyncEvent) -> Unit = {},
    canPush: () -> Boolean = { true },
    onPermanentError: (Throwable) -> Unit = {},
    pushFn: suspend (ByteArray) -> PushResult,
  ): SyncEngine =
    SyncEngine(
      editor = editor,
      documentId = "doc1",
      initialServerHeads = initialServerHeads,
      initialDurableHeads = initialDurableHeads,
      store = store,
      pushFn = pushFn,
      scope = CoroutineScope(coroutineContext),
      isPermanent = isPermanent,
      onEvent = onEvent,
      canPush = canPush,
      onPermanentError = onPermanentError,
      now = { clock++ },
    )

  @Test
  fun confirmedHeadAdvanceWakesProtectionWaiter() = runTest {
    val engine =
      engine(FakeSyncEditor(), FakeDeltaStore()) { PushResult(heads = enc(), durableHeads = enc()) }
    val observed = engine.protectionGeneration
    val waiter = async { engine.awaitProtectionAfter(observed) }

    engine.setConfirmedHeads(enc(1))

    assertTrue(waiter.await())
    engine.stop()
  }

  @Test
  fun identicalConfirmedHeadsDoNotAdvanceProtectionGeneration() = runTest {
    val engine =
      engine(FakeSyncEditor(), FakeDeltaStore()) { PushResult(heads = enc(), durableHeads = enc()) }
    engine.setConfirmedHeads(enc(1))
    val observed = engine.protectionGeneration

    engine.setConfirmedHeads(enc(1))

    assertEquals(observed, engine.protectionGeneration)
    engine.stop()
  }

  @Test
  fun localCaptureWakesProtectionWaiter() = runTest {
    val syncEditor = FakeSyncEditor()
    val engine =
      engine(syncEditor, FakeDeltaStore()) { PushResult(heads = enc(), durableHeads = enc()) }
    runCurrent()
    syncEditor.known.add(1)
    val observed = engine.protectionGeneration
    val waiter = async { engine.awaitProtectionAfter(observed) }

    engine.captureNow()

    assertTrue(waiter.await())
    engine.stop()
  }

  @Test
  fun durableHeadAdvanceDoesNotWakeProtectionWaiter() = runTest {
    val engine =
      engine(FakeSyncEditor(), FakeDeltaStore()) { PushResult(heads = enc(), durableHeads = enc()) }
    val observed = engine.protectionGeneration
    val waiter = async { engine.awaitProtectionAfter(observed) }

    engine.setDurableHeads(enc(1))
    runCurrent()

    assertFalse(waiter.isCompleted)
    waiter.cancel()
    engine.stop()
  }

  @Test
  fun stopTerminatesProtectionWaiterWithoutAdvancingProof() = runTest {
    val engine =
      engine(FakeSyncEditor(), FakeDeltaStore()) { PushResult(heads = enc(), durableHeads = enc()) }
    val observed = engine.protectionGeneration
    val waiter = async { engine.awaitProtectionAfter(observed) }

    engine.stop()

    assertFalse(waiter.await())
    assertEquals(observed, engine.protectionGeneration)
  }

  @Test
  fun checkpointReturnsFailureWhenFrontierInspectionFails() = runTest {
    val failure = IllegalStateException("editor unavailable")
    val baseEditor = FakeSyncEditor()
    var failInspection = false
    val editor =
      object : SyncEditor by baseEditor {
        override suspend fun missingChangesetsFor(confirmedHeads: ByteArray): MissingBytes {
          if (failInspection) throw failure
          return baseEditor.missingChangesetsFor(confirmedHeads)
        }
      }
    val engine =
      engine(editor, FakeDeltaStore()) { PushResult(heads = enc(1), durableHeads = enc()) }
    runCurrent()
    baseEditor.known.add(1)
    failInspection = true

    val checkpoint = engine.checkpointCurrentFrontier()

    assertEquals(failure, checkpoint.exceptionOrNull())
    engine.stop()
  }

  @Test
  fun checkpointReturnsAfterLocalCaptureWhileLivePushIsPending() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    val pushStarted = CompletableDeferred<Unit>()
    val releasePush = CompletableDeferred<Unit>()
    val engine =
      engine(editor, store) {
        pushStarted.complete(Unit)
        releasePush.await()
        PushResult(heads = enc(1), durableHeads = enc())
      }
    runCurrent()
    editor.known.add(1)

    val checkpoint = async { engine.checkpointCurrentFrontier() }
    runCurrent()

    assertTrue(pushStarted.isCompleted)
    assertTrue(checkpoint.await().isSuccess)
    assertEquals(listOf("1"), store.load("doc1").map { it.id })

    releasePush.complete(Unit)
    advanceUntilIdle()
    engine.stop()
  }

  @Test
  fun checkpointReturnsAfterLiveAckWhileLocalCaptureIsPending() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    val captureStarted = CompletableDeferred<Unit>()
    val releaseCapture = CompletableDeferred<Unit>()
    store.onPut = { record ->
      captureStarted.complete(Unit)
      releaseCapture.await()
      store.defaultPut(record)
    }
    val engine = engine(editor, store) { PushResult(heads = enc(1), durableHeads = enc()) }
    runCurrent()
    editor.known.add(1)

    val checkpoint = async { engine.checkpointCurrentFrontier() }
    runCurrent()

    assertTrue(captureStarted.isCompleted)
    assertTrue(checkpoint.await().isSuccess)
    assertFalse(releaseCapture.isCompleted)

    releaseCapture.complete(Unit)
    advanceUntilIdle()
    engine.stop()
  }

  @Test
  fun checkpointAcceptsExistingExactConfirmedFrontierWithoutStartingWork() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var puts = 0
    var pushes = 0
    store.onPut = { puts++ }
    val engine =
      engine(editor, store) {
        pushes++
        PushResult(heads = enc(), durableHeads = enc())
      }
    runCurrent()
    editor.known.add(1)
    engine.setConfirmedHeads(enc(1))

    val startedAt = testScheduler.currentTime
    assertTrue(engine.checkpointCurrentFrontier().isSuccess)
    assertEquals(startedAt, testScheduler.currentTime)
    assertEquals(0, puts)
    assertEquals(0, pushes)
    engine.stop()
  }

  @Test
  fun checkpointAcceptsNewLiveAckWhenCaptureFailsAndDurableHeadsLag() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var puts = 0
    store.onPut = {
      puts++
      throw IllegalStateException("disk unavailable")
    }
    val engine = engine(editor, store) { PushResult(heads = enc(1), durableHeads = enc()) }
    runCurrent()
    editor.known.add(1)

    val checkpoint = engine.checkpointCurrentFrontier()

    assertTrue(checkpoint.isSuccess)
    assertTrue(puts >= 1)
    engine.stop()
  }

  @Test
  fun checkpointRejectsStaleLiveAckAndRetriesCaptureOnce() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var puts = 0
    store.onPut = {
      puts++
      throw IllegalStateException("disk unavailable")
    }
    val engine = engine(editor, store) { PushResult(heads = enc(), durableHeads = enc()) }
    runCurrent()
    editor.known.add(1)

    val checkpoint = engine.checkpointCurrentFrontier()

    assertTrue(checkpoint.isFailure)
    assertEquals(2, puts)
    engine.stop()
  }

  @Test
  fun checkpointRetriesOnlyExplicitCaptureFailure() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var loads = 0
    store.onLoad = {
      loads++
      emptyList()
    }
    val engine = engine(editor, store) { PushResult(heads = enc(), durableHeads = enc()) }
    runCurrent()
    loads = 0
    editor.known.add(1)
    editor.withheld = 1

    val checkpoint = engine.checkpointCurrentFrontier()

    assertTrue(checkpoint.isFailure)
    assertEquals(1, loads)
    engine.stop()
  }

  @Test
  fun checkpointWaitsForPendingLiveAckAfterBothCaptureAttemptsFail() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    val pushStarted = CompletableDeferred<Unit>()
    val releasePush = CompletableDeferred<Unit>()
    var puts = 0
    store.onPut = {
      puts++
      throw IllegalStateException("disk unavailable")
    }
    val engine =
      engine(editor, store) {
        pushStarted.complete(Unit)
        releasePush.await()
        PushResult(heads = enc(1), durableHeads = enc())
      }
    runCurrent()
    editor.known.add(1)

    val checkpoint = async { engine.checkpointCurrentFrontier() }
    runCurrent()

    assertTrue(pushStarted.isCompleted)
    assertEquals(2, puts)
    assertFalse(checkpoint.isCompleted)

    releasePush.complete(Unit)
    assertTrue(checkpoint.await().isSuccess)
    engine.stop()
  }

  @Test
  fun checkpointWaitsForPendingCaptureAfterLivePushFails() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    val captureStarted = CompletableDeferred<Unit>()
    val releaseCapture = CompletableDeferred<Unit>()
    store.onPut = { record ->
      captureStarted.complete(Unit)
      releaseCapture.await()
      store.defaultPut(record)
    }
    val engine = engine(editor, store) { throw IllegalStateException("network unavailable") }
    runCurrent()
    editor.known.add(1)

    val checkpoint = async { engine.checkpointCurrentFrontier() }
    runCurrent()

    assertTrue(captureStarted.isCompleted)
    assertFalse(checkpoint.isCompleted)

    releaseCapture.complete(Unit)
    assertTrue(checkpoint.await().isSuccess)
    engine.stop()
  }

  @Test
  fun checkpointFailsAfterBothDurabilityPathsTerminate() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var puts = 0
    store.onPut = {
      puts++
      throw IllegalStateException("disk unavailable")
    }
    val engine = engine(editor, store) { throw IllegalStateException("network unavailable") }
    runCurrent()
    editor.known.add(1)

    val checkpoint = engine.checkpointCurrentFrontier()

    assertTrue(checkpoint.isFailure)
    assertEquals(2, puts)
    engine.stop()
  }

  @Test
  fun staleStoreDeltaIsAdoptedAndPushed() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    store.records.add(DeltaRecord(id = "7", documentId = "doc1", changeset = enc(7), createdAt = 1))
    store.records.add(DeltaRecord(id = "8", documentId = "doc1", changeset = enc(8), createdAt = 2))
    val pushed = mutableListOf<ByteArray>()
    val engine =
      engine(editor, store) { changesets ->
        pushed.add(changesets)
        editor.receiveRemoteChangeset(changesets)
        val heads = editor.currentHeads()
        PushResult(heads = heads, durableHeads = heads)
      }
    engine.flushNow()
    assertEquals(listOf(7, 8), pushed.flatMap { dec(it) }.sorted())
    engine.stop()
  }

  @Test
  fun durableHeadsWithUnknownConcurrentDotNeverThrows() = runTest {
    val editor = FakeSyncEditor(listOf(1, 2))
    val store = FakeDeltaStore()
    val engine = engine(editor, store) { PushResult(heads = enc(2), durableHeads = enc(2)) }
    engine.setDurableHeads(enc(2, 999))
    engine.flushNow()
    engine.stop()
  }

  @Test
  fun recordIsNotPrunedUntilItsOpIsInDurableHeads() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    val engine = engine(editor, store) { PushResult(heads = enc(5), durableHeads = enc()) }
    engine.flushNow()
    advanceUntilIdle()
    assertEquals(listOf("5"), store.load("doc1").map { it.id })
    engine.setDurableHeads(enc(5))
    advanceUntilIdle()
    assertEquals(0, store.load("doc1").size)
    engine.stop()
  }

  @Test
  fun ackedOpIsNotRePushedEveryCycle() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var pushes = 0
    val engine =
      engine(editor, store) {
        pushes++
        PushResult(heads = enc(5), durableHeads = enc())
      }
    engine.flushNow()
    engine.flushNow()
    engine.flushNow()
    assertEquals(1, pushes)
    engine.stop()
  }

  @Test
  fun captureAppendsNewChangesetByFirstOpDotId() = runTest {
    val editor = FakeSyncEditor(listOf(1, 2))
    val store = FakeDeltaStore()
    val engine =
      engine(editor, store, initialServerHeads = enc(2), initialDurableHeads = enc(2)) {
        val heads = editor.currentHeads()
        PushResult(heads = heads, durableHeads = heads)
      }
    advanceUntilIdle()
    editor.known.add(3)
    engine.captureNow()
    assertEquals(listOf("3"), store.load("doc1").map { it.id })
    engine.stop()
  }

  @Test
  fun pruneDeletesOnlyRecordsProvenServerDurableAndLocal() = runTest {
    val editor = FakeSyncEditor(listOf(1, 2, 3))
    val store = FakeDeltaStore()
    val engine = engine(editor, store) { PushResult(heads = enc(3), durableHeads = enc(3)) }
    engine.flushNow()
    advanceUntilIdle()
    assertEquals(0, store.load("doc1").size)
    engine.stop()
  }

  @Test
  fun engineWithoutOpInGraphNeverPrunesSiblingCrashDurableRecord() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "30", documentId = "doc1", changeset = enc(30), createdAt = 1)
    )
    val engine = engine(editor, store) { PushResult(heads = enc(), durableHeads = enc()) }
    engine.setDurableHeads(enc())
    advanceUntilIdle()
    assertEquals(listOf("30"), store.load("doc1").map { it.id })
    engine.stop()
  }

  @Test
  fun schedulePersistsImmediatelyDecoupledFromPushCadence() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var pushes = 0
    val engine =
      engine(editor, store) {
        pushes++
        PushResult(heads = editor.currentHeads(), durableHeads = enc())
      }
    runCurrent()
    val pushesAfterInit = pushes

    editor.known.add(9)
    engine.schedule()

    assertEquals(listOf("9"), store.load("doc1").map { it.id })
    assertEquals(pushesAfterInit, pushes)

    advanceTimeBy(500)
    runCurrent()
    assertEquals(pushesAfterInit + 1, pushes)
    engine.stop()
  }

  @Test
  fun opAppliedWhilePersistWriteIsInFlightIsNotSwallowedByCapturedHeads() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var injected = false
    store.onPut = { record ->
      store.defaultPut(record)
      if (!injected) {
        injected = true
        editor.known.add(11)
      }
    }
    val engine =
      engine(editor, store) { PushResult(heads = editor.currentHeads(), durableHeads = enc()) }
    runCurrent()

    editor.known.add(10)
    engine.schedule()
    runCurrent()
    engine.schedule()
    runCurrent()

    assertEquals(listOf("10", "11"), store.load("doc1").map { it.id }.sortedBy { it.toInt() })
    engine.stop()
  }

  @Test
  fun failingStoreWriteDoesNotKillLaterPersistsAndFailedDeltaIsRetried() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var failOnce = true
    store.onPut = { record ->
      if (failOnce) {
        failOnce = false
        throw RuntimeException("quota")
      }
      store.defaultPut(record)
    }
    val engine =
      engine(editor, store) { PushResult(heads = editor.currentHeads(), durableHeads = enc()) }
    runCurrent()

    editor.known.add(10)
    engine.schedule()
    runCurrent()
    engine.schedule()
    runCurrent()

    assertEquals(listOf("10"), store.load("doc1").map { it.id })
    engine.stop()
  }

  @Test
  fun persistentlyFailingCaptureDoesNotBlockPushOfSealedChangesets() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    store.onPut = { throw RuntimeException("quota") }
    val pushed = mutableListOf<ByteArray>()
    val engine =
      engine(editor, store) { changesets ->
        pushed.add(changesets)
        PushResult(heads = enc(5), durableHeads = enc(5))
      }
    runCurrent()
    assertEquals(listOf(5), pushed.flatMap { dec(it) })
    assertEquals(SyncStatus.Retrying, engine.status.value)
    engine.stop()
  }

  @Test
  fun repeatedCaptureFailuresAreObservableAndResetOnceCaptureRecovers() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var failing = true
    store.onPut = { record ->
      if (failing) throw RuntimeException("quota")
      store.defaultPut(record)
    }
    var pushes = 0
    val engine =
      engine(editor, store) {
        pushes++
        PushResult(heads = enc(5), durableHeads = enc())
      }
    runCurrent()
    assertEquals(1, engine.captureFailures.value)
    assertEquals(SyncStatus.Retrying, engine.status.value)

    engine.retryNow()
    runCurrent()
    assertEquals(2, engine.captureFailures.value)
    assertEquals(SyncStatus.Retrying, engine.status.value)

    failing = false
    engine.retryNow()
    runCurrent()
    assertEquals(0, engine.captureFailures.value)
    assertEquals(SyncStatus.Idle, engine.status.value)
    assertEquals(1, pushes)
    assertEquals(listOf("5"), store.load("doc1").map { it.id })
    engine.stop()
  }

  @Test
  fun withheldDoesNotAdvanceCapturedHeadsAndOnlyEmittedBytesAreStored() = runTest {
    val editor = FakeSyncEditor(listOf(1, 2))
    val store = FakeDeltaStore()
    val engine =
      engine(editor, store, initialServerHeads = enc(2), initialDurableHeads = enc(2)) {
        PushResult(heads = editor.currentHeads(), durableHeads = enc(2))
      }
    runCurrent()

    editor.known.add(3)
    editor.known.add(4)
    editor.withheld = 1
    editor.missingCalls.clear()
    engine.captureNow()
    engine.captureNow()

    assertEquals(listOf(listOf(2), listOf(2)), editor.missingCalls)
    assertEquals(listOf("3"), store.load("doc1").map { it.id })
    engine.stop()
  }

  @Test
  fun withheldSkipsPruneForTheCycle() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var deleteCalls = 0
    store.onDeleteMany = { _, _ -> deleteCalls++ }
    val engine = engine(editor, store) { PushResult(heads = enc(5), durableHeads = enc()) }
    engine.flushNow()
    advanceUntilIdle()
    val deleteCallsBefore = deleteCalls

    editor.withheld = 1
    engine.setDurableHeads(enc(5))
    runCurrent()

    assertEquals(deleteCallsBefore, deleteCalls)
    assertEquals(listOf("5"), store.load("doc1").map { it.id })
    engine.stop()
  }

  @Test
  fun withheldStillPushesEmittedPrefixAndSurfacesSignal() = runTest {
    val editor = FakeSyncEditor(listOf(1, 2))
    val store = FakeDeltaStore()
    val events = mutableListOf<SyncEvent>()
    val pushed = mutableListOf<ByteArray>()
    val engine =
      engine(
        editor,
        store,
        initialServerHeads = enc(2),
        initialDurableHeads = enc(2),
        onEvent = { events.add(it) },
      ) { changesets ->
        pushed.add(changesets)
        PushResult(heads = enc(2), durableHeads = enc(2))
      }
    runCurrent()

    editor.known.add(3)
    editor.known.add(4)
    editor.withheld = 1
    engine.flushNow()

    assertEquals(listOf(3), pushed.flatMap { dec(it) })
    assertTrue(events.any { it is SyncEvent.PersistWithheld && it.count == 1 })
    engine.stop()
  }

  @Test
  fun backoffIsLinearAndCapped() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var pushes = 0
    val engine =
      engine(editor, store) {
        pushes++
        throw RuntimeException("network")
      }
    runCurrent()
    assertEquals(1, pushes)
    assertEquals(1, engine.retryAttempt.value)

    advanceTimeBy(1999)
    runCurrent()
    assertEquals(1, pushes)
    advanceTimeBy(1)
    runCurrent()
    assertEquals(2, pushes)
    assertEquals(2, engine.retryAttempt.value)

    advanceTimeBy(4000)
    runCurrent()
    assertEquals(3, pushes)

    while (engine.retryAttempt.value < 16) {
      advanceTimeBy(minOf(2000L * engine.retryAttempt.value, 30_000L))
      runCurrent()
    }
    val pushesAtCap = pushes
    advanceTimeBy(29_999)
    runCurrent()
    assertEquals(pushesAtCap, pushes)
    advanceTimeBy(1)
    runCurrent()
    assertEquals(pushesAtCap + 1, pushes)
    engine.stop()
  }

  @Test
  fun permanentFailureStopsPushLoopButPersistKeepsRunning() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var pushes = 0
    val engine =
      engine(editor, store, isPermanent = { true }) {
        pushes++
        throw RuntimeException("invalid_changeset_payload")
      }
    runCurrent()
    assertEquals(1, pushes)
    assertEquals(SyncStatus.Error, engine.status.value)

    editor.known.add(6)
    engine.schedule()
    runCurrent()
    assertEquals(listOf("5", "6"), store.load("doc1").map { it.id }.sorted())

    advanceTimeBy(60_000)
    runCurrent()
    assertEquals(1, pushes)
    engine.stop()
  }

  @Test
  fun continuousSchedulingFiresPushAtMaxWait() = runTest {
    val editor = FakeSyncEditor(listOf(1))
    val store = FakeDeltaStore()
    var pushes = 0
    val engine =
      engine(editor, store) {
        pushes++
        PushResult(heads = editor.currentHeads(), durableHeads = editor.currentHeads())
      }
    runCurrent()
    val pushesAfterInit = pushes

    var next = 2
    repeat(8) {
      editor.known.add(next++)
      engine.schedule()
      advanceTimeBy(400)
      runCurrent()
    }
    assertEquals(pushesAfterInit + 1, pushes)
    engine.stop()
  }

  @Test
  fun gatedPushDoesNotSendButStillPersistsAndFlushesWhenGateOpens() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    var gateOpen = false
    var pushes = 0
    val engine =
      engine(editor, store, canPush = { gateOpen }) {
        pushes++
        PushResult(heads = editor.currentHeads(), durableHeads = enc())
      }
    runCurrent()
    val base = pushes

    editor.known.add(9)
    engine.schedule()
    advanceTimeBy(500)
    runCurrent()

    // 로컬 스태시는 보존되지만 push는 시도되지 않는다.
    assertEquals(listOf("9"), store.load("doc1").map { it.id })
    assertEquals(base, pushes)

    gateOpen = true
    engine.resumePush()
    advanceUntilIdle()
    assertEquals(base + 1, pushes)
    engine.stop()
  }

  @Test
  fun permanentErrorInvokesCallbackAndResumeClearsErrorState() = runTest {
    val editor = FakeSyncEditor()
    val store = FakeDeltaStore()
    val permanentErrors = mutableListOf<Throwable>()
    var fail = true
    val engine =
      engine(
        editor,
        store,
        isPermanent = { true },
        onPermanentError = { permanentErrors.add(it) },
      ) {
        if (fail) throw SyncWsException("subscription_required", true)
        PushResult(heads = editor.currentHeads(), durableHeads = enc())
      }
    runCurrent()

    editor.known.add(3)
    engine.schedule()
    advanceTimeBy(500)
    runCurrent()

    assertEquals(SyncStatus.Error, engine.status.value)
    assertEquals(1, permanentErrors.size)
    assertTrue(isPermanentSyncError(permanentErrors.first()))

    fail = false
    engine.resumePush()
    advanceUntilIdle()
    assertEquals(SyncStatus.Idle, engine.status.value)
    engine.stop()
  }

  @Test
  fun resumePushDuringBackoffRetryingClearsStateAndResumes() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var fail = true
    var pushes = 0
    val engine =
      engine(editor, store) {
        pushes++
        if (fail) throw RuntimeException("network")
        PushResult(heads = enc(5), durableHeads = enc(5))
      }
    runCurrent()

    assertEquals(1, pushes)
    assertEquals(SyncStatus.Retrying, engine.status.value)

    // 백오프 대기 중(재시도 시각 도달 전).
    advanceTimeBy(500)
    runCurrent()
    assertEquals(1, pushes)

    fail = false
    engine.resumePush()
    advanceUntilIdle()

    assertEquals(2, pushes)
    assertEquals(SyncStatus.Idle, engine.status.value)
    assertEquals(0, store.load("doc1").size)
    engine.stop()
  }
}
