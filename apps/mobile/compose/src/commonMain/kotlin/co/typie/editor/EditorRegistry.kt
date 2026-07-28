package co.typie.editor

import co.typie.editor.ffi.ResourceUpdate
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

object EditorRegistry {
  private val mutex = Mutex()
  private val editors = mutableSetOf<Editor>()
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
  private var latestResourceUpdate: ResourceUpdate? = null

  suspend fun register(editor: Editor) {
    mutex.withLock {
      if (!editors.add(editor)) return@withLock
      try {
        latestResourceUpdate?.let(editor::receiveResourceUpdate)
      } catch (e: Throwable) {
        editors.remove(editor)
        throw e
      }
    }
  }

  suspend fun unregister(editor: Editor) {
    mutex.withLock { editors.remove(editor) }
  }

  fun unregisterAsync(editor: Editor) {
    scope.launch { unregister(editor) }
  }

  internal suspend fun snapshot(): List<Editor> = mutex.withLock { editors.toList() }

  internal suspend fun commitResourceUpdate(commit: () -> ResourceUpdate?) {
    mutex.lock()
    try {
      withContext(NonCancellable + Dispatchers.Default) {
        val update = commit() ?: return@withContext
        latestResourceUpdate = update

        val iterator = editors.iterator()
        while (iterator.hasNext()) {
          val editor = iterator.next()
          try {
            editor.receiveResourceUpdate(update)
          } catch (_: Throwable) {
            iterator.remove()
          }
        }
      }
    } finally {
      mutex.unlock()
    }
  }
}
