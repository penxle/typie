package co.typie.editor

import co.touchlab.kermit.Logger
import co.typie.editor.ffi.Editor as FfiEditor
import co.typie.editor.ffi.FrameKey
import co.typie.editor.ffi.Revision
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.collections.immutable.PersistentList
import kotlinx.collections.immutable.PersistentMap
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.launch

internal data class SurfaceConfiguration(
  val width: Double,
  val height: Double,
  val scaleFactor: Double,
)

@OptIn(ExperimentalAtomicApi::class)
internal class SurfaceSessionHandle
internal constructor(private val editor: Editor, internal val id: Long, internal val page: Int) {
  private val retired = AtomicBoolean(false)

  internal val isRetired: Boolean
    get() = retired.load()

  suspend fun requestResize(configuration: SurfaceConfiguration) {
    editor.replaceSurface(this, configuration)
  }

  fun detach(onDetached: () -> Unit = {}) {
    editor.detachSurface(this, onDetached)
  }

  internal fun retire() {
    retired.store(true)
  }
}

internal fun runSurfaceCleanup(cleanup: () -> Unit) {
  try {
    cleanup()
  } catch (_: CancellationException) {
    // Teardown is already complete from the editor's perspective.
  } catch (error: Throwable) {
    Logger.w(error) { "Surface cleanup failed" }
  }
}

private sealed interface SurfaceCommand {
  val session: SurfaceSessionHandle
}

private data class AttachSurface(
  override val session: SurfaceSessionHandle,
  val handle: Long,
  val configuration: SurfaceConfiguration,
) : SurfaceCommand

private data class ResizeSurface(
  override val session: SurfaceSessionHandle,
  val configuration: SurfaceConfiguration,
) : SurfaceCommand

private data class RenderSurface(
  override val session: SurfaceSessionHandle,
  val revision: Long,
  val complete: (FrameKey?) -> Unit,
) : SurfaceCommand

private data class DetachSurface(
  override val session: SurfaceSessionHandle,
  val complete: () -> Unit,
) : SurfaceCommand

/**
 * Native surface effect boundary. Publication facts and render scheduling stay in [Editor]; this
 * driver only preserves native operation order and returns the exact prepared frame identity.
 */
@OptIn(ExperimentalAtomicApi::class)
internal class SurfaceDriver(
  private val inner: FfiEditor,
  private val scope: CoroutineScope,
  private val dispatcher: CoroutineDispatcher,
  private val disposed: AtomicBoolean,
  private val failed: AtomicBoolean,
  private val notifyFailure: (Throwable) -> Unit,
) {
  private val nextSessionId = AtomicLong(0L)
  private val sessions: AtomicReference<PersistentMap<Int, Long>> =
    AtomicReference(persistentMapOf())
  private val commands: AtomicReference<PersistentList<SurfaceCommand>> =
    AtomicReference(persistentListOf())
  private val scheduled = AtomicBoolean(false)
  // Native attachment bookkeeping only; target/proof/requirement facts remain in Editor.
  private val attached = mutableMapOf<Int, Long>()

  fun attach(
    editor: Editor,
    page: Int,
    handle: Long,
    configuration: SurfaceConfiguration,
  ): SurfaceSessionHandle {
    val session =
      SurfaceSessionHandle(editor = editor, id = nextSessionId.addAndFetch(1), page = page)
    if (disposed.load() || failed.load()) return session

    sessions.updatePersistent { it.putting(page, session.id) }
    enqueue(AttachSurface(session, handle, configuration))
    return session
  }

  fun resize(session: SurfaceSessionHandle, configuration: SurfaceConfiguration) {
    if (!isCurrent(session) || disposed.load() || failed.load()) return
    enqueue(ResizeSurface(session, configuration))
  }

  fun render(session: SurfaceSessionHandle, revision: Long, complete: (FrameKey?) -> Unit) {
    val command = RenderSurface(session, revision, complete)
    if (!isCurrent(session) || disposed.load() || failed.load()) {
      completeRender(command, null)
      return
    }
    if (!enqueue(command)) {
      completeRender(command, null)
    }
  }

  fun detach(session: SurfaceSessionHandle, complete: () -> Unit = {}) {
    session.retire()
    removeIfCurrent(session)
    enqueue(DetachSurface(session, complete))
  }

  fun dispose() {
    sessions.store(persistentMapOf())
    discardPendingCommands().filterIsInstance<RenderSurface>().forEach { completeRender(it, null) }
    if (commands.load().isNotEmpty()) schedule()
  }

  private fun enqueue(command: SurfaceCommand): Boolean {
    if ((disposed.load() || failed.load()) && command !is DetachSurface) return false
    commands.updatePersistent { it.adding(command) }
    schedule()
    return true
  }

  private fun schedule() {
    if (!scheduled.compareAndSet(expectedValue = false, newValue = true)) return
    // Editor disposal is allowed to cancel the owner scope immediately after queuing detach.
    // Keep this one existing drain alive long enough to finish callbacks and native cleanup.
    scope.launch(dispatcher + NonCancellable) {
      while (true) {
        val batch = commands.exchange(persistentListOf())
        for (command in batch) {
          try {
            run(command, attached)
          } catch (_: CancellationException) {
            // A cancelled operation must not skip later render/detach completions in this batch.
          } catch (error: Throwable) {
            notifyFailure(error)
          }
        }

        scheduled.store(false)
        if (commands.load().isEmpty()) return@launch
        if (!scheduled.compareAndSet(expectedValue = false, newValue = true)) return@launch
      }
    }
  }

  private fun run(command: SurfaceCommand, attached: MutableMap<Int, Long>) {
    if ((disposed.load() || failed.load()) && command !is DetachSurface) {
      if (command is RenderSurface) completeRender(command, null)
      return
    }
    if (command is DetachSurface) {
      runDetach(command, attached)
      return
    }
    try {
      when (command) {
        is AttachSurface -> {
          if (!isCurrent(command.session)) return
          inner.attachSurface(
            command.session.page,
            command.handle,
            command.configuration.width,
            command.configuration.height,
            command.configuration.scaleFactor,
          )
          attached[command.session.page] = command.session.id
        }
        is ResizeSurface -> {
          if (!isCurrent(command.session) || attached[command.session.page] != command.session.id) {
            return
          }
          inner.resizeSurface(
            command.session.page,
            command.configuration.width,
            command.configuration.height,
            command.configuration.scaleFactor,
          )
        }
        is RenderSurface -> runRender(command, attached)
        is DetachSurface -> error("handled above")
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      notifyFailure(e)
    }
  }

  private fun runRender(command: RenderSurface, attached: MutableMap<Int, Long>) {
    if (!isCurrent(command.session) || attached[command.session.page] != command.session.id) {
      completeRender(command, null)
      return
    }
    val frameKey =
      try {
        inner.renderSurface(command.session.page, Revision(command.revision))
      } catch (error: Throwable) {
        completeRender(command, null, error)
        return
      }
    completeRender(command, frameKey)
  }

  private fun completeRender(
    command: RenderSurface,
    frameKey: FrameKey?,
    operationFailure: Throwable? = null,
  ) {
    var failure = operationFailure
    try {
      command.complete(frameKey)
    } catch (completionFailure: Throwable) {
      if (failure == null) {
        failure = completionFailure
      } else {
        failure.addSuppressed(completionFailure)
      }
    }
    when (val error = failure) {
      null -> Unit
      is CancellationException -> throw error
      else -> notifyFailure(error)
    }
  }

  private fun runDetach(command: DetachSurface, attached: MutableMap<Int, Long>) {
    if (attached[command.session.page] == command.session.id) {
      runSurfaceCleanup {
        try {
          inner.detachSurface(command.session.page)
        } finally {
          attached.remove(command.session.page)
        }
      }
    }
    runSurfaceCleanup(command.complete)
  }

  private fun isCurrent(session: SurfaceSessionHandle): Boolean =
    sessions.load()[session.page] == session.id

  private fun removeIfCurrent(session: SurfaceSessionHandle) {
    sessions.updatePersistent { current ->
      if (current[session.page] == session.id) current.removing(session.page) else current
    }
  }

  private fun discardPendingCommands(): List<SurfaceCommand> {
    while (true) {
      val current = commands.load()
      val retained = current.filterIsInstance<DetachSurface>().toPersistentList()
      if (commands.compareAndSet(current, retained)) {
        return current.filterNot { it is DetachSurface }
      }
    }
  }

  private inline fun <T> AtomicReference<T>.updatePersistent(transform: (T) -> T): T {
    while (true) {
      val current = load()
      val next = transform(current)
      if (compareAndSet(current, next)) return next
    }
  }
}
