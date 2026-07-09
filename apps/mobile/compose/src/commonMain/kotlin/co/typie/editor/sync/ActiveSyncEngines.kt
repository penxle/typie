package co.typie.editor.sync

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

internal val syncAppScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

data class SyncSession(val engine: SyncEngine, val pipeline: RemoteChangesetPipeline)

object ActiveSyncEngines {
  private val engines = mutableMapOf<String, MutableSet<SyncSession>>()

  fun register(documentId: String, session: SyncSession) {
    engines.getOrPut(documentId) { mutableSetOf() }.add(session)
  }

  fun unregister(documentId: String, session: SyncSession) {
    val set = engines[documentId] ?: return
    set.remove(session)
    if (set.isEmpty()) engines.remove(documentId)
  }

  fun openDocumentIds(): Set<String> = engines.keys.toSet()

  private fun all(): List<SyncSession> = engines.values.flatten()

  suspend fun flushAll() {
    for (session in all()) {
      catchingNonCancellation { session.engine.flushNow() }
    }
  }

  fun retryAll() {
    for (session in all()) {
      session.engine.retryNow()
    }
  }

  fun stopAll() {
    for (session in all()) {
      session.pipeline.stop()
      session.engine.stop()
    }
    engines.clear()
  }
}
