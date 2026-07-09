package co.typie.editor.sync

import co.typie.sync.db.SyncDatabase
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.IO
import kotlinx.coroutines.withContext

data class DeltaRecord(
  val id: String,
  val documentId: String,
  val changeset: ByteArray,
  val createdAt: Long,
)

interface DeltaStore {
  suspend fun load(documentId: String): List<DeltaRecord>

  suspend fun put(record: DeltaRecord)

  suspend fun deleteMany(documentId: String, ids: List<String>)

  suspend fun listDocumentIds(): List<String>

  suspend fun wipeAll()
}

class SqlDeltaStore(private val database: SyncDatabase) : DeltaStore {
  override suspend fun load(documentId: String): List<DeltaRecord> =
    withContext(Dispatchers.IO) {
      database.pendingChangesetQueries.loadByDocument(documentId).executeAsList().map {
        DeltaRecord(
          id = it.id,
          documentId = it.documentId,
          changeset = it.changeset,
          createdAt = it.createdAt,
        )
      }
    }

  override suspend fun put(record: DeltaRecord) {
    withContext(Dispatchers.IO) {
      database.pendingChangesetQueries.upsert(
        documentId = record.documentId,
        id = record.id,
        changeset = record.changeset,
        createdAt = record.createdAt,
      )
    }
  }

  override suspend fun deleteMany(documentId: String, ids: List<String>) {
    withContext(Dispatchers.IO) {
      database.pendingChangesetQueries.transaction {
        for (id in ids) {
          database.pendingChangesetQueries.deleteById(documentId, id)
        }
      }
    }
  }

  override suspend fun listDocumentIds(): List<String> =
    withContext(Dispatchers.IO) {
      database.pendingChangesetQueries.listDocumentIds().executeAsList()
    }

  override suspend fun wipeAll() {
    withContext(Dispatchers.IO) { database.pendingChangesetQueries.wipeAll() }
  }
}

object ChangesetDeltaStore : DeltaStore by SqlDeltaStore(SyncDatabase(createSyncDatabaseDriver()))
