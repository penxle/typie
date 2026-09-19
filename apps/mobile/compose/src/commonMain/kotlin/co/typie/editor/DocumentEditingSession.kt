package co.typie.editor

import androidx.compose.runtime.snapshotFlow
import co.typie.editor.sync.ActiveDocumentEditingSessions
import co.typie.editor.sync.ChangesetDeltaStore
import co.typie.editor.sync.RemoteChangesetPipeline
import co.typie.editor.sync.SyncEngine
import co.typie.editor.sync.asSyncEditor
import co.typie.editor.sync.isPermanentSyncError
import co.typie.editor.sync.ws.DocumentSyncBaseline
import co.typie.editor.sync.ws.DocumentWsChannel
import co.typie.editor.sync.ws.SyncWs
import co.typie.editor.sync.ws.WsSyncTransport
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicInt
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.coroutines.CoroutineContext
import kotlin.time.Clock
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
import kotlinx.coroutines.selects.select

internal sealed interface EditingCheckpointResult {
  data object Protected : EditingCheckpointResult

  data class EditFailed(val cause: Throwable) : EditingCheckpointResult

  data class ProtectionFailed(val cause: Throwable) : EditingCheckpointResult

  data object StopCancelled : EditingCheckpointResult

  data object SessionStopped : EditingCheckpointResult
}

internal enum class DocumentSaveState {
  Pending,
  Failed,
  Protected,
}

internal interface DocumentEditingStop {
  suspend fun awaitCheckpoint(): EditingCheckpointResult

  suspend fun retryCheckpoint(): EditingCheckpointResult

  fun cancel()
}

/** Owns its sync scope and resources; the runtime retains ownership of the editor core. */
@OptIn(ExperimentalAtomicApi::class)
internal class DocumentEditingSession(
  val documentId: String,
  val editor: Editor,
  private val engine: SyncEngine,
  private val pipeline: RemoteChangesetPipeline,
  private val scope: CoroutineScope,
) {
  companion object {
    fun create(
      documentId: String,
      editor: Editor,
      baseline: DocumentSyncBaseline,
      channel: DocumentWsChannel,
      onReload: suspend (DocumentEditingSession, Boolean) -> Unit,
      canPush: () -> Boolean = { true },
      onPermanentError: (Throwable) -> Unit = {},
    ): DocumentEditingSession {
      val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
      try {
        lateinit var session: DocumentEditingSession
        val transport =
          WsSyncTransport(
            channel = channel,
            connection = SyncWs.connection,
            documentId = documentId,
            onReload = { onReload(session, true) },
          )
        // SyncEngine starts capture/push here. Reload callbacks start only after the
        // screen attaches this session and calls start() for the remote pipeline.
        val engine =
          SyncEngine(
            editor = editor.asSyncEditor(),
            documentId = documentId,
            initialServerHeads = baseline.heads,
            initialDurableHeads = baseline.durableHeads,
            store = ChangesetDeltaStore,
            pushFn = transport::push,
            scope = scope,
            isPermanent = ::isPermanentSyncError,
            canPush = canPush,
            onPermanentError = onPermanentError,
            now = { Clock.System.now().toEpochMilliseconds() },
          )
        val pipeline =
          RemoteChangesetPipeline(
            editor = editor.asSyncEditor(),
            headsSink = engine,
            transport = transport,
            initialSeq = baseline.seq,
            scope = scope,
            onNeedsReload = { onReload(session, false) },
          )
        session = DocumentEditingSession(documentId, editor, engine, pipeline, scope)
        return session
      } catch (throwable: Throwable) {
        scope.cancel()
        throw throwable
      }
    }
  }

  private sealed interface State {
    data object Active : State

    data object StartingStop : State

    class PreparingStop(val preparation: StopPreparation) : State

    class ResumingStop(val preparation: StopPreparation) : State

    data object Closed : State
  }

  private inner class StopPreparation(private val quiescence: LocalEditQuiescence) {
    private val owners = AtomicInt(1)
    private val stopped = CompletableDeferred<Unit>()
    private val editResult: Deferred<Result<Unit>> =
      scope.async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }
    private val initialAttempt: Deferred<EditingCheckpointResult> =
      scope.async(start = CoroutineStart.UNDISPATCHED) { runCheckpoint() }
    private val retryAttempt = AtomicReference<Deferred<EditingCheckpointResult>?>(null)

    val stoppedSignal: Deferred<Unit>
      get() = stopped

    val isStopped: Boolean
      get() = stopped.isCompleted

    fun retain(): Boolean {
      while (true) {
        val current = owners.load()
        if (current <= 0) return false
        if (owners.compareAndSet(current, current + 1)) return true
      }
    }

    fun release(): Boolean {
      while (true) {
        val current = owners.load()
        if (current <= 0) return false
        val next = current - 1
        if (owners.compareAndSet(current, next)) return next == 0
      }
    }

    fun resume() {
      quiescence.resume()
    }

    fun stop() {
      if (!stopped.complete(Unit)) return
      editResult.cancel()
      initialAttempt.cancel()
      retryAttempt.load()?.cancel()
    }

    fun initialCheckpoint(): Deferred<EditingCheckpointResult> = initialAttempt

    fun retryCheckpoint(): Deferred<EditingCheckpointResult> {
      if (!initialAttempt.isCompleted) return initialAttempt
      while (true) {
        if (isStopped) {
          return CompletableDeferred(EditingCheckpointResult.SessionStopped)
        }
        val current = retryAttempt.load()
        if (current != null && !current.isCompleted) return current
        val next = scope.async(start = CoroutineStart.LAZY) { runCheckpoint() }
        if (retryAttempt.compareAndSet(current, next)) {
          next.start()
          return next
        }
        next.cancel()
      }
    }

    private suspend fun runCheckpoint(): EditingCheckpointResult {
      if (isStopped) return EditingCheckpointResult.SessionStopped
      val settledEdits =
        try {
          editResult.await()
        } catch (e: CancellationException) {
          if (isStopped) return EditingCheckpointResult.SessionStopped
          throw e
        }
      if (isStopped) return EditingCheckpointResult.SessionStopped
      val checkpoint = engine.checkpointCurrentFrontier()
      return resolveResult(settledEdits, checkpoint)
    }

    private fun resolveResult(
      editResult: Result<Unit>,
      checkpointResult: Result<Unit>,
    ): EditingCheckpointResult {
      if (isStopped) return EditingCheckpointResult.SessionStopped
      editResult.exceptionOrNull()?.let {
        return EditingCheckpointResult.EditFailed(it)
      }
      checkpointResult.exceptionOrNull()?.let {
        return EditingCheckpointResult.ProtectionFailed(it)
      }
      return EditingCheckpointResult.Protected
    }
  }

  private inner class StopHandle(private val preparation: StopPreparation) : DocumentEditingStop {
    private val active = AtomicBoolean(true)
    private val cancelled = CompletableDeferred<Unit>()

    override suspend fun awaitCheckpoint(): EditingCheckpointResult =
      await(preparation.initialCheckpoint())

    override suspend fun retryCheckpoint(): EditingCheckpointResult =
      terminalResult() ?: await(preparation.retryCheckpoint())

    override fun cancel() {
      if (!active.compareAndSet(expectedValue = true, newValue = false)) return
      cancelled.complete(Unit)
      release(preparation)
    }

    private suspend fun await(attempt: Deferred<EditingCheckpointResult>): EditingCheckpointResult {
      terminalResult()?.let {
        return it
      }
      val result =
        try {
          select<EditingCheckpointResult> {
            preparation.stoppedSignal.onAwait { EditingCheckpointResult.SessionStopped }
            cancelled.onAwait { EditingCheckpointResult.StopCancelled }
            attempt.onAwait { it }
          }
        } catch (e: CancellationException) {
          terminalResult()?.let {
            return it
          }
          throw e
        }
      return terminalResult() ?: result
    }

    private fun terminalResult(): EditingCheckpointResult? =
      when {
        preparation.isStopped -> EditingCheckpointResult.SessionStopped
        !active.load() -> EditingCheckpointResult.StopCancelled
        else -> null
      }
  }

  private val state = AtomicReference<State>(State.Active)
  private val started = AtomicBoolean(false)
  private val revisionJob = AtomicReference<Job?>(null)

  fun <T : Job> submit(start: (Editor, CoroutineContext) -> T): T? {
    if (state.load() !== State.Active) return null
    return editor.trackLocalEdit { context -> start(editor, context) }
  }

  fun beginStop(): DocumentEditingStop {
    while (true) {
      when (val current = state.load()) {
        State.Active -> {
          if (!state.compareAndSet(current, State.StartingStop)) continue
          val preparation = StopPreparation(editor.quiesceLocalEdits())
          if (!state.compareAndSet(State.StartingStop, State.PreparingStop(preparation))) {
            preparation.stop()
          }
          return StopHandle(preparation)
        }
        State.StartingStop -> continue
        is State.PreparingStop -> {
          if (current.preparation.retain()) return StopHandle(current.preparation)
        }
        is State.ResumingStop -> continue
        State.Closed -> error("Document editing session is not active")
      }
    }
  }

  internal val protectionGeneration: Long
    get() = engine.protectionGeneration

  internal suspend fun awaitProtectionAfter(observedGeneration: Long): Boolean =
    engine.awaitProtectionAfter(observedGeneration)

  internal val isStopped: Boolean
    get() = state.load() === State.Closed

  internal suspend fun awaitStopped() = engine.awaitStopped()

  suspend fun awaitProtectedCheckpoint(
    stop: DocumentEditingStop,
    onStateChange: (DocumentSaveState) -> Unit = {},
  ): EditingCheckpointResult = coroutineScope {
    val checkpoint = MutableStateFlow<EditingCheckpointResult?>(null)
    val progress =
      launch(start = CoroutineStart.UNDISPATCHED) {
        combine(checkpoint, engine.saveInProgress) { result, saving ->
            when (result) {
              EditingCheckpointResult.Protected -> DocumentSaveState.Protected
              is EditingCheckpointResult.EditFailed -> DocumentSaveState.Failed
              is EditingCheckpointResult.ProtectionFailed ->
                if (saving) DocumentSaveState.Pending else DocumentSaveState.Failed
              else -> DocumentSaveState.Pending
            }
          }
          .distinctUntilChanged()
          .collect(onStateChange)
      }
    try {
      var result: EditingCheckpointResult
      do {
        val observedGeneration = protectionGeneration
        checkpoint.value = null
        result = stop.retryCheckpoint()
        checkpoint.value = result
        if (
          result is EditingCheckpointResult.ProtectionFailed &&
            !awaitProtectionAfter(observedGeneration)
        ) {
          result = EditingCheckpointResult.SessionStopped
        }
      } while (result is EditingCheckpointResult.ProtectionFailed)
      result
    } finally {
      progress.cancel()
    }
  }

  fun start() {
    check(state.load() === State.Active) { "Document editing session is not active" }
    if (!started.compareAndSet(expectedValue = false, newValue = true)) return
    ActiveDocumentEditingSessions.register(this)
    pipeline.start()
    if (state.load() !== State.Active) {
      pipeline.stop()
      if (isStopped) ActiveDocumentEditingSessions.unregister(this)
      return
    }

    val job =
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        snapshotFlow { editor.appliedState.documentRevision }.drop(1).collect { engine.schedule() }
      }
    revisionJob.store(job)
    if (state.load() !== State.Active) {
      revisionJob.exchange(null)?.cancel()
      pipeline.stop()
      if (isStopped) ActiveDocumentEditingSessions.unregister(this)
    }
  }

  fun retrySyncNow() {
    engine.retryNow()
  }

  /** 재구독/백스톱 회복 시 permanent Error를 포함해 push를 재개한다. */
  fun resumeSyncNow() {
    engine.resumePush()
  }

  suspend fun flushSyncNow() {
    engine.flushNow()
  }

  fun stop() {
    val previous = state.exchange(State.Closed)
    if (previous === State.Closed) return
    ActiveDocumentEditingSessions.unregister(this)
    when (previous) {
      is State.PreparingStop -> previous.preparation.stop()
      is State.ResumingStop -> previous.preparation.stop()
      else -> {}
    }
    revisionJob.exchange(null)?.cancel()
    pipeline.stop()
    engine.stop()
    scope.cancel()
  }

  private fun release(preparation: StopPreparation) {
    if (!preparation.release()) return
    while (true) {
      val current = state.load()
      if (current !is State.PreparingStop || current.preparation !== preparation) return
      val resuming = State.ResumingStop(preparation)
      if (!state.compareAndSet(current, resuming)) continue
      preparation.resume()
      state.compareAndSet(resuming, State.Active)
      return
    }
  }
}
