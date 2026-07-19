package co.typie.editor.sync

import co.typie.editor.DocumentEditingSession
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

internal val syncAppScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)

internal object ActiveDocumentEditingSessions {
  private val sessions = mutableMapOf<String, MutableSet<DocumentEditingSession>>()

  fun register(session: DocumentEditingSession) {
    sessions.getOrPut(session.documentId) { mutableSetOf() }.add(session)
  }

  fun unregister(session: DocumentEditingSession) {
    val set = sessions[session.documentId] ?: return
    set.remove(session)
    if (set.isEmpty()) sessions.remove(session.documentId)
  }

  fun openDocumentIds(): Set<String> = sessions.keys.toSet()

  private fun all(): List<DocumentEditingSession> = sessions.values.flatten()

  suspend fun flushSyncAll() {
    for (session in all()) {
      catchingNonCancellation { session.flushSyncNow() }
    }
  }

  fun retrySyncAll() {
    for (session in all()) {
      session.retrySyncNow()
    }
  }

  fun resumeSyncAll() {
    for (session in all()) {
      session.resumeSyncNow()
    }
  }

  fun stopAll() {
    for (session in all()) {
      session.stop()
    }
    sessions.clear()
  }
}
