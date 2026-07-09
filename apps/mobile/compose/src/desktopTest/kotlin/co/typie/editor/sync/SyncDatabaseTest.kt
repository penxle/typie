package co.typie.editor.sync

import app.cash.sqldelight.driver.jdbc.sqlite.JdbcSqliteDriver
import co.typie.sync.db.SyncDatabase
import kotlin.test.Test
import kotlin.test.assertEquals

class SyncDatabaseTest {
  private fun inMemoryDatabase(): SyncDatabase {
    val driver = JdbcSqliteDriver(JdbcSqliteDriver.IN_MEMORY)
    SyncDatabase.Schema.create(driver)
    return SyncDatabase(driver)
  }

  @Test
  fun upsertAndLoadRoundTrips() {
    val db = inMemoryDatabase()
    db.pendingChangesetQueries.upsert("doc1", "1:5", byteArrayOf(1, 2, 3), 100L)
    val rows = db.pendingChangesetQueries.loadByDocument("doc1").executeAsList()
    assertEquals(1, rows.size)
    assertEquals("1:5", rows[0].id)
  }

  @Test
  fun sameIdInDifferentDocumentsDoesNotCollide() {
    val db = inMemoryDatabase()
    db.pendingChangesetQueries.upsert("doc1", "1:0", byteArrayOf(1), 100L)
    db.pendingChangesetQueries.upsert("doc2", "1:0", byteArrayOf(2), 100L)
    assertEquals(2, db.pendingChangesetQueries.listDocumentIds().executeAsList().size)
  }

  @Test
  fun upsertPreservesRowidOrderOnSameCreatedAt() {
    val db = inMemoryDatabase()
    db.pendingChangesetQueries.upsert("doc1", "1:1", byteArrayOf(1), 100L)
    db.pendingChangesetQueries.upsert("doc1", "1:2", byteArrayOf(2), 100L)
    db.pendingChangesetQueries.upsert("doc1", "1:1", byteArrayOf(9), 100L)
    assertEquals(
      listOf("1:1", "1:2"),
      db.pendingChangesetQueries.loadByDocument("doc1").executeAsList().map { it.id },
    )
  }

  @Test
  fun reopeningPersistentDatabaseDoesNotFail() {
    val path = java.io.File.createTempFile("typie-sync-test", ".db").absolutePath
    createDesktopSyncDriver(path).close()
    val driver = createDesktopSyncDriver(path)
    SyncDatabase(driver).pendingChangesetQueries.upsert("doc1", "1:1", byteArrayOf(1), 1L)
    driver.close()
  }
}
