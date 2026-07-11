package co.typie.editor.sync

import co.touchlab.kermit.Logger
import co.typie.editor.sync.ws.SyncWs
import kotlinx.coroutines.sync.Mutex
import okio.ByteString.Companion.toByteString

class OrphanSweeper(
  private val store: DeltaStore,
  private val pushFn: suspend (documentId: String, payload: ByteArray) -> Unit,
  private val openDocumentIds: () -> Set<String>,
) {
  private val mutex = Mutex()
  private val sweptFingerprints = mutableMapOf<String, Map<String, String>>()
  private val permanentFailures = mutableMapOf<String, Map<String, String>>()

  fun resetPermanentFailures() {
    permanentFailures.clear()
  }

  suspend fun sweep(includeOpenDocuments: Boolean = false, deleteOnSuccess: Boolean = false) {
    if (!mutex.tryLock()) return
    try {
      val excluded = if (includeOpenDocuments) emptySet() else openDocumentIds()
      val documentIds = store.listDocumentIds() - excluded
      for (documentId in documentIds) {
        catchingNonCancellation {
          val records = store.load(documentId)
          if (records.isEmpty()) return@catchingNonCancellation
          val fingerprint = records.associate {
            it.id to it.changeset.toByteString().sha256().hex()
          }
          if (sweptFingerprints[documentId] == fingerprint) return@catchingNonCancellation
          if (permanentFailures[documentId] == fingerprint) return@catchingNonCancellation
          pushFn(documentId, records.map { it.changeset }.concatChangesets())
          sweptFingerprints[documentId] = fingerprint
          if (deleteOnSuccess) store.deleteMany(documentId, records.map { it.id })
        }
          .onFailure {
            if (isPermanentSyncError(it)) {
              val records = store.load(documentId)
              permanentFailures[documentId] = records.associate { r ->
                r.id to r.changeset.toByteString().sha256().hex()
              }
            }
            Logger.w(it) { "OrphanSweeper: push failed for $documentId" }
          }
      }
    } finally {
      mutex.unlock()
    }
  }
}

val orphanSweeper: OrphanSweeper by lazy {
  OrphanSweeper(
    store = ChangesetDeltaStore,
    pushFn = { documentId, payload -> SyncWs.connection.push(documentId, payload) },
    openDocumentIds = { ActiveSyncEngines.openDocumentIds() },
  )
}
