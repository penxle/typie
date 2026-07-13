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

internal sealed interface LocalDurabilityResult {
  data object Captured : LocalDurabilityResult

  data class EditFailed(val cause: Throwable) : LocalDurabilityResult

  data class CaptureFailed(val cause: Throwable) : LocalDurabilityResult

  data object SessionStopped : LocalDurabilityResult
}

internal interface DocumentEditingClose {
  suspend fun awaitLocalDurability(): LocalDurabilityResult

  suspend fun retryLocalDurability(): LocalDurabilityResult

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
    private inner class Attempt {
      private val result = CompletableDeferred<LocalDurabilityResult>()
      private val work =
        scope.launch(start = CoroutineStart.LAZY) {
          if (state.load() === State.Closed) {
            result.complete(LocalDurabilityResult.SessionStopped)
            return@launch
          }
          val settledEdits =
            try {
              editResult.await()
            } catch (e: CancellationException) {
              if (state.load() === State.Closed) {
                result.complete(LocalDurabilityResult.SessionStopped)
                return@launch
              }
              throw e
            }
          val captureResult =
            try {
              engine.captureNow()
              Result.success(Unit)
            } catch (e: CancellationException) {
              throw e
            } catch (e: Throwable) {
              Result.failure(e)
            }

          result.complete(resolveResult(settledEdits, captureResult))
        }

      fun start() {
        work.start()
      }

      suspend fun await(): LocalDurabilityResult = result.await()

      fun stop() {
        if (result.complete(LocalDurabilityResult.SessionStopped)) {
          work.cancel()
        }
      }
    }

    private val editResult: Deferred<Result<Unit>> =
      scope.async(start = CoroutineStart.UNDISPATCHED) { quiescence.await() }
    private val attempts = AtomicReference(Attempt().also(Attempt::start))

    override suspend fun awaitLocalDurability(): LocalDurabilityResult = attempts.load().await()

    override suspend fun retryLocalDurability(): LocalDurabilityResult {
      while (true) {
        val current = attempts.load()
        val currentResult = current.await()
        if (currentResult !is LocalDurabilityResult.CaptureFailed) return currentResult
        val currentState = state.load()
        if (currentState === State.Closed) return LocalDurabilityResult.SessionStopped
        if (currentState !is State.Closing || currentState.close !== this) return currentResult

        val next = Attempt()
        if (attempts.compareAndSet(current, next)) {
          next.start()
          return next.await()
        }
      }
    }

    override fun cancel() {
      val current = state.load()
      if (current !is State.Closing || current.close !== this) return
      if (!state.compareAndSet(current, State.Active)) return
      quiescence.resume()
    }

    fun stop() {
      editResult.cancel()
      attempts.load().stop()
    }

    private fun resolveResult(
      editResult: Result<Unit>,
      captureResult: Result<Unit>,
    ): LocalDurabilityResult {
      if (state.load() === State.Closed) return LocalDurabilityResult.SessionStopped
      captureResult.exceptionOrNull()?.let {
        return LocalDurabilityResult.CaptureFailed(it)
      }
      editResult.exceptionOrNull()?.let {
        return LocalDurabilityResult.EditFailed(it)
      }
      return LocalDurabilityResult.Captured
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
