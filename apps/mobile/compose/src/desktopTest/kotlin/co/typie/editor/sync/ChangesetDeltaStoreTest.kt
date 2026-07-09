package co.typie.editor.sync

import app.cash.sqldelight.driver.jdbc.sqlite.JdbcSqliteDriver
import co.typie.sync.db.SyncDatabase
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest

class ChangesetDeltaStoreTest {
  private fun store(): SqlDeltaStore {
    val driver = JdbcSqliteDriver(JdbcSqliteDriver.IN_MEMORY)
    SyncDatabase.Schema.create(driver)
    return SqlDeltaStore(SyncDatabase(driver))
  }

  @Test
  fun loadReturnsRecordsInInsertionOrder() = runTest {
    val s = store()
    s.put(
      DeltaRecord(id = "1:2", documentId = "doc1", changeset = byteArrayOf(2), createdAt = 200L)
    )
    s.put(
      DeltaRecord(id = "1:1", documentId = "doc1", changeset = byteArrayOf(1), createdAt = 100L)
    )
    s.put(
      DeltaRecord(id = "1:3", documentId = "doc1", changeset = byteArrayOf(3), createdAt = 100L)
    )
    assertEquals(listOf("1:1", "1:3", "1:2"), s.load("doc1").map { it.id })
  }

  @Test
  fun putIsUpsert() = runTest {
    val s = store()
    s.put(
      DeltaRecord(id = "1:1", documentId = "doc1", changeset = byteArrayOf(1), createdAt = 100L)
    )
    s.put(
      DeltaRecord(id = "1:1", documentId = "doc1", changeset = byteArrayOf(9), createdAt = 100L)
    )
    val records = s.load("doc1")
    assertEquals(1, records.size)
    assertContentEquals(byteArrayOf(9), records[0].changeset)
  }

  @Test
  fun deleteManyIsScopedToDocument() = runTest {
    val s = store()
    s.put(
      DeltaRecord(id = "1:0", documentId = "doc1", changeset = byteArrayOf(1), createdAt = 100L)
    )
    s.put(
      DeltaRecord(id = "1:0", documentId = "doc2", changeset = byteArrayOf(2), createdAt = 100L)
    )
    s.deleteMany("doc1", listOf("1:0"))
    assertEquals(0, s.load("doc1").size)
    assertEquals(1, s.load("doc2").size)
  }

  @Test
  fun listDocumentIdsAndWipeAll() = runTest {
    val s = store()
    s.put(
      DeltaRecord(id = "1:1", documentId = "doc1", changeset = byteArrayOf(1), createdAt = 100L)
    )
    s.put(
      DeltaRecord(id = "1:1", documentId = "doc2", changeset = byteArrayOf(2), createdAt = 100L)
    )
    assertEquals(setOf("doc1", "doc2"), s.listDocumentIds().toSet())
    s.wipeAll()
    assertEquals(0, s.listDocumentIds().size)
  }
}
