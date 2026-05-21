package co.typie.editor

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

internal object EditorLocalChangesetBus {
  private val mutex = Mutex()
  private val pending = mutableMapOf<String, MutableList<ByteArray>>()
  private val notifications = MutableSharedFlow<String>(extraBufferCapacity = 16)

  suspend fun publish(entityId: String, changesets: ByteArray) {
    mutex.withLock { pending.getOrPut(entityId) { mutableListOf() }.add(changesets) }
    notifications.emit(entityId)
  }

  suspend fun consume(entityId: String): List<ByteArray> = mutex.withLock {
    pending.remove(entityId).orEmpty()
  }

  fun notifications(entityId: String): Flow<Unit> = notifications.filter { it == entityId }.map {}
}
