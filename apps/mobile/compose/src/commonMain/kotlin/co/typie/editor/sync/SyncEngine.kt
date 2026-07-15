package co.typie.editor.sync

import co.touchlab.kermit.Logger
import kotlin.concurrent.Volatile
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.selects.select

data class PushResult(val heads: ByteArray, val durableHeads: ByteArray)

enum class SyncStatus {
  Idle,
  Pushing,
  Retrying,
  Error,
}

sealed interface SyncEvent {
  data class PushFired(val bytes: Int) : SyncEvent

  data class PushSuccess(val durationMs: Long) : SyncEvent

  data class PushError(val message: String) : SyncEvent

  data class PersistWithheld(val count: Int) : SyncEvent
}

interface SyncHeadsSink {
  fun setConfirmedHeads(heads: ByteArray)

  fun setDurableHeads(heads: ByteArray)
}

class SyncEngine(
  private val editor: SyncEditor,
  private val documentId: String,
  initialServerHeads: ByteArray,
  initialDurableHeads: ByteArray,
  private val store: DeltaStore,
  private val pushFn: suspend (ByteArray) -> PushResult,
  private val scope: CoroutineScope,
  private val isPermanent: (Throwable) -> Boolean = { false },
  private val onEvent: (SyncEvent) -> Unit = {},
  private val now: () -> Long,
) : SyncHeadsSink {
  private companion object {
    const val IDLE_MS = 500L
    const val MAX_WAIT_MS = 3000L
    const val BACKOFF_BASE_MS = 2000L
    const val BACKOFF_CAP_MS = 30_000L
    const val DORMANT_ADOPT_LIMIT = 8
  }

  @Volatile private var confirmedHeads = initialServerHeads
  @Volatile private var durableHeads = initialDurableHeads
  @Volatile private var capturedHeads = initialServerHeads
  private val blockedCount = mutableMapOf<String, Int>()
  private val dormant = mutableSetOf<String>()
  private var inflight = false
  private var persistTail: Deferred<Result<Unit>> = CompletableDeferred(Result.success(Unit))
  private var persistQueued = false
  private var pushTail: Deferred<Result<Unit>> = CompletableDeferred(Result.success(Unit))
  private var pushQueued = false
  private val flushWaiters = mutableListOf<CompletableDeferred<Unit>>()
  private var flushAfterInflight = false
  private var idleJob: Job? = null
  private var maxWaitJob: Job? = null
  private var retryJob: Job? = null
  @Volatile private var stopped = false

  private val statusFlow = MutableStateFlow(SyncStatus.Idle)
  val status: StateFlow<SyncStatus> = statusFlow
  private val retryAttemptFlow = MutableStateFlow(0)
  val retryAttempt: StateFlow<Int> = retryAttemptFlow
  private val captureFailuresFlow = MutableStateFlow(0)
  val captureFailures: StateFlow<Int> = captureFailuresFlow

  init {
    scope.launch(start = CoroutineStart.UNDISPATCHED) { firePush() }
  }

  private suspend fun capture() {
    if (stopped) return
    val records = store.load(documentId)
    val localAll = editor.changesetIds().toSet()
    for (record in records) {
      if (record.id in localAll) {
        blockedCount.remove(record.id)
        dormant.remove(record.id)
        continue
      }
      if (record.id in dormant) continue
      val partitioned = editor.partitionRemoteChangesets(record.changeset)
      if (partitioned.ready.isNotEmpty()) {
        editor.receiveRemoteChangeset(partitioned.ready)
        blockedCount.remove(record.id)
      } else {
        val count = (blockedCount[record.id] ?: 0) + 1
        blockedCount[record.id] = count
        if (count >= DORMANT_ADOPT_LIMIT) dormant.add(record.id)
      }
    }
    persistFresh().await().getOrThrow()
  }

  private fun persistFresh(): Deferred<Result<Unit>> {
    if (persistQueued) return persistTail
    persistQueued = true
    val previous = persistTail
    val run =
      scope.async(start = CoroutineStart.UNDISPATCHED) {
        previous.await()
        persistQueued = false
        catchingNonCancellation {
          if (stopped) return@catchingNonCancellation
          val heads = editor.currentHeads()
          val missing = editor.missingChangesetsFor(capturedHeads)
          if (missing.bytes.isNotEmpty()) {
            for (entry in editor.splitChangesets(missing.bytes)) {
              store.put(
                DeltaRecord(
                  id = entry.id,
                  documentId = documentId,
                  changeset = entry.bytes,
                  createdAt = now(),
                )
              )
            }
          }
          if (missing.withheld > 0) {
            onEvent(SyncEvent.PersistWithheld(missing.withheld))
          } else {
            capturedHeads = heads
          }
        }
      }
    persistTail = run
    return run
  }

  private suspend fun drain() {
    if (stopped) return
    val missing = editor.missingChangesetsFor(confirmedHeads)
    if (missing.withheld > 0) onEvent(SyncEvent.PersistWithheld(missing.withheld))
    if (missing.bytes.isEmpty()) return
    onEvent(SyncEvent.PushFired(missing.bytes.size))
    val result = pushFn(missing.bytes)
    setConfirmedHeads(result.heads)
    setDurableHeads(result.durableHeads)
  }

  private fun pushFresh(): Deferred<Result<Unit>> {
    if (pushQueued) return pushTail
    pushQueued = true
    val previous = pushTail
    val run =
      scope.async(start = CoroutineStart.UNDISPATCHED) {
        previous.await()
        pushQueued = false
        catchingNonCancellation {
          if (stopped) return@catchingNonCancellation
          drain()
        }
      }
    pushTail = run
    return run
  }

  private suspend fun frontierCoveredBy(heads: ByteArray): Boolean {
    val missing = editor.missingChangesetsFor(heads)
    return missing.bytes.isEmpty() && missing.withheld == 0
  }

  private suspend fun currentFrontierIsProtected(): Boolean =
    frontierCoveredBy(capturedHeads) || frontierCoveredBy(confirmedHeads)

  private fun captureWithOneRetry(): Deferred<Result<Unit>> =
    scope.async(start = CoroutineStart.UNDISPATCHED) {
      val first = catchingNonCancellation { capture() }
      if (currentFrontierIsProtected()) return@async Result.success(Unit)
      if (first.isSuccess) return@async first

      val retry = catchingNonCancellation { capture() }
      if (currentFrontierIsProtected()) Result.success(Unit) else retry
    }

  private fun checkpointFailure(
    captureResult: Result<Unit>,
    pushResult: Result<Unit>,
  ): Result<Unit> {
    val captureFailure = captureResult.exceptionOrNull()
    val pushFailure = pushResult.exceptionOrNull()
    val failure =
      captureFailure
        ?: pushFailure
        ?: IllegalStateException("Current editor frontier is not protected")
    if (captureFailure != null && pushFailure != null && pushFailure !== failure) {
      failure.addSuppressed(pushFailure)
    }
    return Result.failure(failure)
  }

  private suspend fun prune() {
    if (stopped) return
    val missing = editor.missingChangesetsFor(durableHeads)
    if (missing.withheld > 0) return
    val localAll = editor.changesetIds().toSet()
    val stillMissing = editor.splitChangesets(missing.bytes).map { it.id }.toSet()
    val durableSet = localAll - stillMissing
    val records = store.load(documentId)
    val toDelete = records.filter { it.id in durableSet }.map { it.id }
    store.deleteMany(documentId, toDelete)
  }

  private fun clearScheduleTimers() {
    idleJob?.cancel()
    idleJob = null
    maxWaitJob?.cancel()
    maxWaitJob = null
  }

  private fun clearTimers() {
    clearScheduleTimers()
    retryJob?.cancel()
    retryJob = null
  }

  private fun flushScheduledChanges() {
    clearScheduleTimers()
    if (inflight) {
      flushAfterInflight = true
      return
    }
    if (statusFlow.value == SyncStatus.Retrying) return
    scope.launch(start = CoroutineStart.UNDISPATCHED) { firePush() }
  }

  private suspend fun firePush() {
    if (stopped || statusFlow.value == SyncStatus.Error || inflight) return
    clearTimers()
    inflight = true
    statusFlow.value = SyncStatus.Pushing
    val startedAt = now()
    try {
      var captureError: Throwable? = null
      try {
        capture()
        captureFailuresFlow.value = 0
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        captureError = e
        captureFailuresFlow.value += 1
      }
      if (stopped) return
      pushFresh().await().getOrThrow()
      if (stopped) return
      captureError?.let { throw it }
      finishSuccess(startedAt)
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      if (!stopped) {
        onEvent(SyncEvent.PushError(e.message ?: e.toString()))
        handleFailure(e)
      }
    } finally {
      inflight = false
      flushWaiters.forEach { it.complete(Unit) }
      flushWaiters.clear()
      if (!stopped && flushAfterInflight) {
        flushAfterInflight = false
        flushScheduledChanges()
      }
    }
  }

  private fun finishSuccess(startedAt: Long) {
    if (stopped) return
    statusFlow.value = SyncStatus.Idle
    retryAttemptFlow.value = 0
    onEvent(SyncEvent.PushSuccess(now() - startedAt))
  }

  private fun handleFailure(error: Throwable) {
    if (isPermanent(error)) {
      statusFlow.value = SyncStatus.Error
      Logger.e(error) { "SyncEngine: permanent failure" }
      return
    }
    statusFlow.value = SyncStatus.Retrying
    retryAttemptFlow.value += 1
    val delayMs = minOf(BACKOFF_BASE_MS * retryAttemptFlow.value, BACKOFF_CAP_MS)
    Logger.w {
      "SyncEngine: transient failure (attempt ${retryAttemptFlow.value}), retrying in ${delayMs}ms"
    }
    retryJob = scope.launch {
      delay(delayMs)
      retryJob = null
      firePush()
    }
  }

  fun retryNow() {
    if (stopped) return
    if (inflight) return
    if (statusFlow.value == SyncStatus.Error) return
    retryJob?.cancel()
    retryJob = null
    statusFlow.value = SyncStatus.Idle
    scope.launch(start = CoroutineStart.UNDISPATCHED) { firePush() }
  }

  override fun setConfirmedHeads(heads: ByteArray) {
    confirmedHeads = heads
  }

  override fun setDurableHeads(heads: ByteArray) {
    durableHeads = heads
    scope.launch {
      try {
        prune()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        Logger.w(e) { "SyncEngine: prune failed" }
      }
    }
  }

  suspend fun captureNow() {
    capture()
  }

  suspend fun checkpointCurrentFrontier(): Result<Unit> =
    try {
      checkpointCurrentFrontierUnchecked()
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      Result.failure(e)
    }

  private suspend fun checkpointCurrentFrontierUnchecked(): Result<Unit> {
    if (currentFrontierIsProtected()) return Result.success(Unit)

    val capture = captureWithOneRetry()
    val push = pushFresh()
    var captureResult: Result<Unit>? = null
    var pushResult: Result<Unit>? = null

    while (true) {
      if (currentFrontierIsProtected()) return Result.success(Unit)
      if (captureResult != null && pushResult != null) {
        return checkpointFailure(captureResult, pushResult)
      }

      select {
        if (captureResult == null) {
          capture.onAwait { captureResult = it }
        }
        if (pushResult == null) {
          push.onAwait { pushResult = it }
        }
      }
    }
  }

  suspend fun flushNow() {
    if (inflight) {
      val waiter = CompletableDeferred<Unit>()
      flushWaiters.add(waiter)
      waiter.await()
    }
    capture()
    pushFresh().await().getOrThrow()
  }

  fun schedule() {
    if (stopped) return
    scope.launch(start = CoroutineStart.UNDISPATCHED) {
      persistFresh().await().onFailure {
        Logger.w(it) { "SyncEngine: persist failed, will retry on next edit" }
      }
    }
    if (statusFlow.value == SyncStatus.Error) return

    idleJob?.cancel()
    idleJob = scope.launch {
      delay(IDLE_MS)
      idleJob = null
      flushScheduledChanges()
    }

    if (maxWaitJob == null) {
      maxWaitJob = scope.launch {
        delay(MAX_WAIT_MS)
        maxWaitJob = null
        flushScheduledChanges()
      }
    }
  }

  fun stop() {
    stopped = true
    clearTimers()
  }
}
