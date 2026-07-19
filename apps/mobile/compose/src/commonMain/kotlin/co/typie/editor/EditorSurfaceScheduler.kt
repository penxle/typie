package co.typie.editor

import co.typie.editor.ffi.Editor as FfiEditor
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.AtomicReference
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlinx.collections.immutable.PersistentList
import kotlinx.collections.immutable.PersistentMap
import kotlinx.collections.immutable.PersistentSet
import kotlinx.collections.immutable.persistentListOf
import kotlinx.collections.immutable.persistentMapOf
import kotlinx.collections.immutable.persistentSetOf
import kotlinx.collections.immutable.toPersistentList
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

internal data class SurfaceConfiguration(
  val width: Double,
  val height: Double,
  val scaleFactor: Double,
)

private typealias SurfacePresentedCallback = (version: Long) -> Unit

internal class SurfaceSessionHandle
internal constructor(
  private val scheduler: EditorSurfaceScheduler,
  internal val id: Long,
  internal val page: Int,
) {
  fun requestRender(onPresented: SurfacePresentedCallback) {
    scheduler.requestRender(this, onPresented)
  }

  fun requestResize(configuration: SurfaceConfiguration, onPresented: SurfacePresentedCallback) {
    scheduler.requestResize(this, configuration, onPresented)
  }

  fun detach(onDetached: () -> Unit = {}) {
    scheduler.detachSurface(this, onDetached)
  }
}

private data class SurfaceSession(val id: Long)

private data class AttachedSurfaceSession(val id: Long, val configuration: SurfaceConfiguration)

private data class SurfaceSessionKey(val page: Int, val sessionId: Long)

private sealed interface SurfaceCommand {
  val sessionId: Long
  val page: Int
}

private data class SurfaceAttachCommand(
  override val sessionId: Long,
  override val page: Int,
  val handle: Long,
  val configuration: SurfaceConfiguration,
) : SurfaceCommand

private sealed interface SurfaceRenderCommand : SurfaceCommand {
  val onPresented: SurfacePresentedCallback
}

private data class SurfaceRenderOnlyCommand(
  override val sessionId: Long,
  override val page: Int,
  override val onPresented: SurfacePresentedCallback,
) : SurfaceRenderCommand

private data class SurfaceResizeAndRenderCommand(
  override val sessionId: Long,
  override val page: Int,
  val configuration: SurfaceConfiguration,
  override val onPresented: SurfacePresentedCallback,
) : SurfaceRenderCommand

private data class SurfaceDetachCommand(
  override val sessionId: Long,
  override val page: Int,
  val onDetached: () -> Unit,
) : SurfaceCommand

@OptIn(ExperimentalAtomicApi::class)
internal class EditorSurfaceScheduler(
  private val inner: FfiEditor,
  private val scope: CoroutineScope,
  private val dispatcher: CoroutineDispatcher,
  private val versionCounter: AtomicLong,
  private val disposed: AtomicBoolean,
  private val markPageAttached: (Int) -> Unit,
  private val markPageDetached: (Int) -> Unit,
  private val onPageSettled: (Int, Long) -> Unit,
  private val notifyFailure: (Throwable) -> Unit,
) {
  private val sessionCounter: AtomicLong = AtomicLong(0L)
  private val sessions: AtomicReference<PersistentMap<Int, SurfaceSession>> =
    AtomicReference(persistentMapOf())
  private val commands: AtomicReference<PersistentList<SurfaceCommand>> =
    AtomicReference(persistentListOf())
  private val scheduled: AtomicBoolean = AtomicBoolean(false)
  private val attachedSessions: AtomicReference<PersistentMap<Int, AttachedSurfaceSession>> =
    AtomicReference(persistentMapOf())
  private val pendingAttachSessions: AtomicReference<PersistentSet<SurfaceSessionKey>> =
    AtomicReference(persistentSetOf())

  fun attachSurface(
    page: Int,
    handle: Long,
    width: Double,
    height: Double,
    scaleFactor: Double,
  ): SurfaceSessionHandle {
    val sessionId = sessionCounter.addAndFetch(1L)
    val surface = SurfaceSessionHandle(this, sessionId, page)
    if (disposed.load()) return surface

    sessions.updatePersistent { it.putting(page, SurfaceSession(sessionId)) }
    pendingAttachSessions.updatePersistent { it.adding(SurfaceSessionKey(page, sessionId)) }
    markPageAttached(page)
    enqueue(
      SurfaceAttachCommand(
        sessionId = sessionId,
        page = page,
        handle = handle,
        configuration = SurfaceConfiguration(width, height, scaleFactor),
      )
    )
    return surface
  }

  fun requestRender(surface: SurfaceSessionHandle, onPresented: SurfacePresentedCallback) {
    if (disposed.load() || !isActive(surface)) return
    enqueue(
      SurfaceRenderOnlyCommand(
        sessionId = surface.id,
        page = surface.page,
        onPresented = onPresented,
      )
    )
  }

  fun requestResize(
    surface: SurfaceSessionHandle,
    configuration: SurfaceConfiguration,
    onPresented: SurfacePresentedCallback,
  ) {
    if (disposed.load() || !isActive(surface)) return
    enqueue(
      SurfaceResizeAndRenderCommand(
        sessionId = surface.id,
        page = surface.page,
        configuration = configuration,
        onPresented = onPresented,
      )
    )
  }

  fun detachSurface(surface: SurfaceSessionHandle, onDetached: () -> Unit = {}) {
    if (disposed.load() && !requiresDeferredDetach(surface.page, surface.id)) {
      invokeDetached(onDetached)
      return
    }

    if (!disposed.load() && removeIfCurrent(surface)) {
      markPageDetached(surface.page)
    }
    enqueue(
      SurfaceDetachCommand(sessionId = surface.id, page = surface.page, onDetached = onDetached)
    )
  }

  fun dispose() {
    sessions.store(persistentMapOf())
    val retainedCommands = commands.updatePersistent { queue ->
      queue.filterIsInstance<SurfaceDetachCommand>().toPersistentList()
    }
    if (retainedCommands.isNotEmpty()) {
      schedule()
    }
  }

  private fun removeIfCurrent(surface: SurfaceSessionHandle): Boolean {
    while (true) {
      val current = sessions.load()
      if (current[surface.page]?.id != surface.id) return false
      if (sessions.compareAndSet(current, current.removing(surface.page))) return true
    }
  }

  private fun isActive(surface: SurfaceSessionHandle): Boolean =
    sessions.load()[surface.page]?.id == surface.id

  private fun isActive(page: Int, sessionId: Long): Boolean = sessions.load()[page]?.id == sessionId

  private fun requiresDeferredDetach(page: Int, sessionId: Long): Boolean =
    attachedSessions.load()[page]?.id == sessionId ||
      pendingAttachSessions.load().contains(SurfaceSessionKey(page, sessionId))

  private fun enqueue(command: SurfaceCommand) {
    if (disposed.load() && command !is SurfaceDetachCommand) {
      return
    }
    if (
      disposed.load() &&
        command is SurfaceDetachCommand &&
        !requiresDeferredDetach(command.page, command.sessionId)
    ) {
      invokeDetached(command.onDetached)
      return
    }

    commands.updatePersistent { queue -> queue.coalesce(command) }
    schedule()
  }

  private fun PersistentList<SurfaceCommand>.coalesce(
    command: SurfaceCommand
  ): PersistentList<SurfaceCommand> =
    when (command) {
      is SurfaceAttachCommand -> adding(command)
      is SurfaceRenderOnlyCommand -> coalesceRender(command)
      is SurfaceResizeAndRenderCommand -> coalesceResize(command)
      is SurfaceDetachCommand -> coalesceDetach(command)
    }

  private fun PersistentList<SurfaceCommand>.coalesceRender(
    command: SurfaceRenderOnlyCommand
  ): PersistentList<SurfaceCommand> {
    var replaced = false
    val next =
      map { queued ->
          when {
            queued is SurfaceResizeAndRenderCommand && queued.sessionId == command.sessionId -> {
              replaced = true
              queued.copy(onPresented = command.onPresented)
            }
            queued is SurfaceRenderOnlyCommand && queued.sessionId == command.sessionId -> {
              replaced = true
              command
            }
            else -> queued
          }
        }
        .toPersistentList()
    return if (replaced) next else next.adding(command)
  }

  private fun PersistentList<SurfaceCommand>.coalesceResize(
    command: SurfaceResizeAndRenderCommand
  ): PersistentList<SurfaceCommand> =
    filterNot {
        it.sessionId == command.sessionId &&
          (it is SurfaceRenderOnlyCommand || it is SurfaceResizeAndRenderCommand)
      }
      .toPersistentList()
      .adding(command)

  private fun PersistentList<SurfaceCommand>.coalesceDetach(
    command: SurfaceDetachCommand
  ): PersistentList<SurfaceCommand> =
    filterNot {
        it.sessionId == command.sessionId &&
          (it is SurfaceRenderOnlyCommand || it is SurfaceResizeAndRenderCommand)
      }
      .toPersistentList()
      .adding(command)

  private fun schedule() {
    if (!scheduled.compareAndSet(expectedValue = false, newValue = true)) return
    scope.launch(dispatcher) {
      try {
        drain()
      } catch (e: CancellationException) {
        scheduled.store(false)
        throw e
      } catch (e: Throwable) {
        scheduled.store(false)
        notifyFailure(e)
        if (commands.load().isNotEmpty()) {
          schedule()
        }
      }
    }
  }

  private suspend fun drain() {
    while (true) {
      val batch = commands.exchange(persistentListOf())
      if (batch.isNotEmpty()) {
        flush(batch)
      }

      scheduled.store(false)
      if (commands.load().isEmpty()) return
      if (!scheduled.compareAndSet(expectedValue = false, newValue = true)) return
    }
  }

  private suspend fun flush(batch: PersistentList<SurfaceCommand>) {
    for (command in batch) {
      try {
        when (command) {
          is SurfaceAttachCommand -> flushAttach(command)
          is SurfaceRenderCommand -> flushRender(command)
          is SurfaceDetachCommand -> flushDetach(command)
        }
      } catch (e: CancellationException) {
        throw e
      } catch (e: Throwable) {
        handleFailure(command, e)
      }
    }
  }

  private suspend fun flushAttach(command: SurfaceAttachCommand) {
    val key = SurfaceSessionKey(command.page, command.sessionId)
    if (disposed.load() || !isActive(command.page, command.sessionId)) {
      removePendingAttach(key)
      return
    }

    var attached = false
    if (!disposed.load() && isActive(command.page, command.sessionId)) {
      inner.attachSurface(
        command.page,
        command.handle,
        command.configuration.width,
        command.configuration.height,
        command.configuration.scaleFactor,
      )
      attachedSessions.updatePersistent {
        it.putting(command.page, AttachedSurfaceSession(command.sessionId, command.configuration))
      }
      attached = true
    }

    removePendingAttach(key)
    if (!attached) {
      return
    }

    if (!isActive(command.page, command.sessionId) && !disposed.load()) {
      detachAttachedSession(command.page, command.sessionId)
    }
  }

  private suspend fun flushRender(command: SurfaceRenderCommand) {
    if (disposed.load() || !isActive(command.page, command.sessionId)) return

    val applied =
      when (command) {
        is SurfaceResizeAndRenderCommand -> {
          val configuration = command.configuration
          inner.resizeSurface(
            command.page,
            configuration.width,
            configuration.height,
            configuration.scaleFactor,
          )
          attachedSessions.updatePersistent { sessions ->
            if (sessions[command.page]?.id == command.sessionId) {
              sessions.putting(
                command.page,
                AttachedSurfaceSession(command.sessionId, configuration),
              )
            } else {
              sessions
            }
          }
          configuration
        }

        is SurfaceRenderOnlyCommand -> {
          val attached = attachedSessions.load()[command.page]
          if (attached?.id != command.sessionId) return
          attached.configuration
        }
      }
    val pageSize = inner.pageSizes().getOrNull(command.page)
    if (
      pageSize != null &&
        (pageSize.width.toDouble() != applied.width || pageSize.height.toDouble() != applied.height)
    ) {
      // A tick changed logical page geometry before the matching surface resize was applied.
      // Rendering here would put the new layout into an old target. Settle this attempt;
      // the published state will request the matching resize-and-render.
      onPageSettled(command.page, versionCounter.load())
      return
    }
    // Reading the version before rendering under-reports what the frame may contain,
    // which only delays a settle until the follow-up render command.
    val version = versionCounter.load()
    val presented = inner.renderSurface(command.page)

    if (!isActive(command.page, command.sessionId)) return
    if (presented) {
      command.onPresented(version)
    } else {
      // Skipped render: no bitmap commit will arrive for this page, so the editor
      // settle barrier must be released here.
      onPageSettled(command.page, version)
    }
  }

  private suspend fun flushDetach(command: SurfaceDetachCommand) {
    removePendingAttach(SurfaceSessionKey(command.page, command.sessionId))
    try {
      detachAttachedSession(command.page, command.sessionId)
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      removeAttachedSession(command.page, command.sessionId)
      notifyFailure(e)
    } finally {
      invokeDetached(command.onDetached)
    }
  }

  private suspend fun detachAttachedSession(page: Int, sessionId: Long) {
    if (attachedSessions.load()[page]?.id != sessionId) return
    inner.detachSurface(page)
    removeAttachedSession(page, sessionId)
  }

  private fun removeAttachedSession(page: Int, sessionId: Long) {
    attachedSessions.updatePersistent { current ->
      if (current[page]?.id == sessionId) current.removing(page) else current
    }
  }

  private fun removePendingAttach(key: SurfaceSessionKey) {
    pendingAttachSessions.updatePersistent { it.removing(key) }
  }

  private fun handleFailure(command: SurfaceCommand, error: Throwable) {
    notifyFailure(error)
    when (command) {
      is SurfaceAttachCommand -> {
        removePendingAttach(SurfaceSessionKey(command.page, command.sessionId))
        val surface = SurfaceSessionHandle(this, command.sessionId, command.page)
        if (removeIfCurrent(surface)) {
          markPageDetached(command.page)
        }
      }
      is SurfaceRenderCommand -> onPageSettled(command.page, versionCounter.load())
      is SurfaceDetachCommand -> Unit
    }
  }

  private fun invokeDetached(onDetached: () -> Unit) {
    try {
      onDetached()
    } catch (e: Throwable) {
      notifyFailure(e)
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
