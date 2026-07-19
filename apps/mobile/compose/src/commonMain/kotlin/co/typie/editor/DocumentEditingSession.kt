package co.typie.editor

import androidx.compose.runtime.snapshotFlow
import co.typie.editor.sync.RemoteChangesetPipeline
import co.typie.editor.sync.SyncEngine
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch

internal sealed interface EditingCheckpointResult {
  data object Protected : EditingCheckpointResult

  data class EditFailed(val cause: Throwable) : EditingCheckpointResult

  data class ProtectionFailed(val cause: Throwable) : EditingCheckpointResult

  data object SessionStopped : EditingCheckpointResult
}

internal interface DocumentEditingClose {
  suspend fun awaitCheckpoint(): EditingCheckpointResult

  fun cancel()
}

@OptIn(ExperimentalAtomicApi::class)
internal class DocumentEditingSession(
  val documentId: String,
  val editor: Editor,
  private val engine: SyncEngine,
  private val pipeline: RemoteChangesetPipeline,
  private val scope: CoroutineScope,
) {
  private sealed interface State {
    data object Active : State

    data object StartingClose : State

    class Closing(val close: Close) : State

    data object Closed : State
  }

  private inner class Close(private val quiescence: LocalEditQuiescence) : DocumentEditingClose {
    private val editResult: Deferred<Result<Unit>> =
      scope.async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }
    private val result = CompletableDeferred<EditingCheckpointResult>()
    private val work =
      scope.launch(start = CoroutineStart.LAZY) {
        if (state.load() === State.Closed) {
          result.complete(EditingCheckpointResult.SessionStopped)
          return@launch
        }
        val settledEdits =
          try {
            editResult.await()
          } catch (e: CancellationException) {
            if (state.load() === State.Closed) {
              result.complete(EditingCheckpointResult.SessionStopped)
              return@launch
            }
            throw e
          }
        val checkpoint = engine.checkpointCurrentFrontier()
        result.complete(resolveResult(settledEdits, checkpoint))
      }

    init {
      work.start()
    }

    override suspend fun awaitCheckpoint(): EditingCheckpointResult = result.await()

    override fun cancel() {
      val current = state.load()
      if (current !is State.Closing || current.close !== this) return
      if (!state.compareAndSet(current, State.Active)) return
      quiescence.resume()
    }

    fun stop() {
      editResult.cancel()
      if (result.complete(EditingCheckpointResult.SessionStopped)) work.cancel()
    }

    private fun resolveResult(
      editResult: Result<Unit>,
      checkpointResult: Result<Unit>,
    ): EditingCheckpointResult {
      if (state.load() === State.Closed) return EditingCheckpointResult.SessionStopped
      editResult.exceptionOrNull()?.let {
        return EditingCheckpointResult.EditFailed(it)
      }
      checkpointResult.exceptionOrNull()?.let {
        return EditingCheckpointResult.ProtectionFailed(it)
      }
      return EditingCheckpointResult.Protected
    }
  }

  private val state = AtomicReference<State>(State.Active)
  private val started = AtomicBoolean(false)
  private val revisionJob = AtomicReference<Job?>(null)

  fun <T : Job> submit(start: (Editor, CoroutineContext) -> T): T? {
    if (state.load() !== State.Active) return null
    return editor.trackLocalEdit { context -> start(editor, context) }
  }

  fun beginClose(): DocumentEditingClose {
    val starting = State.StartingClose
    check(state.compareAndSet(State.Active, starting)) { "Document editing session is not active" }
    val close = Close(editor.quiesceLocalEdits())
    if (!state.compareAndSet(starting, State.Closing(close))) {
      close.stop()
    }
    return close
  }

  fun start() {
    check(state.load() === State.Active) { "Document editing session is not active" }
    if (!started.compareAndSet(expectedValue = false, newValue = true)) return
    pipeline.start()
    if (state.load() !== State.Active) {
      pipeline.stop()
      return
    }

    val job =
      scope.launch(start = CoroutineStart.UNDISPATCHED) {
        snapshotFlow { editor.state.documentRevision }.drop(1).collect { engine.schedule() }
      }
    revisionJob.store(job)
    if (state.load() !== State.Active) {
      revisionJob.exchange(null)?.cancel()
      pipeline.stop()
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
    if (previous is State.Closing) previous.close.stop()
    revisionJob.exchange(null)?.cancel()
    pipeline.stop()
    engine.stop()
  }
}
