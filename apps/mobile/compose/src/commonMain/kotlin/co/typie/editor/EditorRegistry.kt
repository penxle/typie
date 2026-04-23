package co.typie.editor

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

object EditorRegistry {
  private val mutex = Mutex()
  private val editors = mutableSetOf<Editor>()
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

  suspend fun register(editor: Editor) {
    mutex.withLock { editors.add(editor) }
  }

  suspend fun unregister(editor: Editor) {
    mutex.withLock { editors.remove(editor) }
  }

  fun unregisterAsync(editor: Editor) {
    scope.launch { unregister(editor) }
  }

  suspend fun snapshot(): List<Editor> = mutex.withLock { editors.toList() }
}
