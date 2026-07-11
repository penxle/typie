package co.typie.editor.sync

import co.typie.editor.Editor
import co.typie.editor.ffi.GraphIngest
import co.typie.editor.ffi.ThemeVariant
import co.typie.editor.ffi.Viewport
import co.typie.editor.sync.ws.DocumentSyncBaseline
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.cancel

@OptIn(ExperimentalAtomicApi::class)
internal class DocumentEditorLoad(
  private val ingest: GraphIngest,
  initialBaseline: DocumentSyncBaseline,
  pending: List<ByteArray>,
  parentScope: CoroutineScope,
  private val onEditorError: (Editor, Throwable) -> Unit,
) {
  val isClosed: Boolean
    get() = closed.load()

  val initialBaseline =
    initialBaseline.copy(
      heads = initialBaseline.heads.copyOf(),
      durableHeads = initialBaseline.durableHeads.copyOf(),
    )

  private val pendingEncoded = encodeLengthPrefixedBlobs(pending.map { it.copyOf() })
  private val job = SupervisorJob(parentScope.coroutineContext[Job])
  private val scope = CoroutineScope(parentScope.coroutineContext + job)
  private val closed = AtomicBoolean(false)
  private val terminalClaimed = AtomicBoolean(false)
  private val creation = AtomicReference<Deferred<Editor>?>(null)
  private val completedEditor = AtomicReference<Editor?>(null)

  init {
    job.invokeOnCompletion { close() }
  }

  suspend fun awaitEditor(viewport: Viewport, themeVariant: ThemeVariant): Editor {
    if (closed.load()) throw closedCancellation()

    val existing = creation.load()
    val deferred = existing ?: startCreation(viewport, themeVariant)
    return deferred.await()
  }

  fun close() {
    if (!closed.compareAndSet(expectedValue = false, newValue = true)) return

    abortIfUnclaimed()
    completedEditor.exchange(null)?.dispose()
    scope.cancel(closedCancellation())
  }

  private fun startCreation(viewport: Viewport, themeVariant: ThemeVariant): Deferred<Editor> {
    val candidate =
      scope.async(start = CoroutineStart.LAZY) {
        try {
          val editor =
            Editor.createInitialized(
              scope = scope,
              themeVariant = themeVariant,
              dispatcher = Dispatchers.Default.limitedParallelism(1),
              onError = onEditorError,
              createInner = {
                if (!terminalClaimed.compareAndSet(expectedValue = false, newValue = true)) {
                  throw closedCancellation()
                }
                ingest.finishWithPending(pendingEncoded, viewport)
              },
            )

          completedEditor.store(editor)
          if (closed.load()) {
            completedEditor.compareAndSet(expectedValue = editor, newValue = null)
            editor.dispose()
            throw closedCancellation()
          }
          editor
        } finally {
          abortIfUnclaimed()
        }
      }

    if (creation.compareAndSet(expectedValue = null, newValue = candidate)) {
      candidate.start()
      return candidate
    }

    candidate.cancel()
    return checkNotNull(creation.load())
  }

  private fun abortIfUnclaimed() {
    if (terminalClaimed.compareAndSet(expectedValue = false, newValue = true)) {
      runCatching { ingest.abort() }
    }
  }

  private fun closedCancellation() = CancellationException("Document editor load closed")
}
