package co.typie.editor.sync

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest

class OrphanSweeperTest {
  @Test
  fun pushesConcatenatedPendingPerDocumentInInsertionOrder() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    store.records.add(
      DeltaRecord(id = "1:2", documentId = "docA", changeset = byteArrayOf(2), createdAt = 2)
    )
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docB", changeset = byteArrayOf(9), createdAt = 3)
    )
    val pushed = mutableMapOf<String, ByteArray>()
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { documentId, payload -> pushed[documentId] = payload },
        openDocumentIds = { emptySet() },
      )

    sweeper.sweep()

    assertContentEquals(byteArrayOf(1, 2), pushed["docA"])
    assertContentEquals(byteArrayOf(9), pushed["docB"])
    assertEquals(3, store.records.size)
  }

  @Test
  fun skipsCurrentlyOpenDocuments() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docB", changeset = byteArrayOf(2), createdAt = 2)
    )
    val pushed = mutableListOf<String>()
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { documentId, _ -> pushed.add(documentId) },
        openDocumentIds = { setOf("docA") },
      )

    sweeper.sweep()

    assertEquals(listOf("docB"), pushed)
  }

  @Test
  fun oneDocumentFailureDoesNotStopOthers() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docB", changeset = byteArrayOf(2), createdAt = 2)
    )
    val pushed = mutableListOf<String>()
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { documentId, _ ->
          if (documentId == "docA") throw RuntimeException("network")
          pushed.add(documentId)
        },
        openDocumentIds = { emptySet() },
      )

    sweeper.sweep()

    assertEquals(listOf("docB"), pushed)
    assertEquals(2, store.records.size)
  }

  @Test
  fun secondSweepWithUnchangedRecordsDoesNotRePush() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    var pushes = 0
    val sweeper =
      OrphanSweeper(store = store, pushFn = { _, _ -> pushes++ }, openDocumentIds = { emptySet() })

    sweeper.sweep()
    sweeper.sweep()
    assertEquals(1, pushes)

    store.records.add(
      DeltaRecord(id = "1:2", documentId = "docA", changeset = byteArrayOf(2), createdAt = 2)
    )
    sweeper.sweep()
    assertEquals(2, pushes)
  }

  @Test
  fun failedSweepIsRetriedOnNextTrigger() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    var fail = true
    var pushes = 0
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { _, _ ->
          if (fail) throw RuntimeException("network")
          pushes++
        },
        openDocumentIds = { emptySet() },
      )

    sweeper.sweep()
    assertEquals(0, pushes)

    fail = false
    sweeper.sweep()
    assertEquals(1, pushes)
  }

  @Test
  fun includeOpenDocumentsSweepsEverything() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    val pushed = mutableListOf<String>()
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { documentId, _ -> pushed.add(documentId) },
        openDocumentIds = { setOf("docA") },
      )

    sweeper.sweep(includeOpenDocuments = true)
    assertEquals(listOf("docA"), pushed)
  }

  @Test
  fun changedBytesForSameIdTriggerRePush() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    var pushes = 0
    val sweeper =
      OrphanSweeper(store = store, pushFn = { _, _ -> pushes++ }, openDocumentIds = { emptySet() })

    sweeper.sweep()
    assertEquals(1, pushes)

    store.records[0] = store.records[0].copy(changeset = byteArrayOf(1, 2))
    sweeper.sweep()
    assertEquals(2, pushes)
  }

  @Test
  fun permanentFailureIsSkippedUntilRecordsChangeOrReset() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    var attempts = 0
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { _, _ ->
          attempts++
          throw com.apollographql.apollo.exception.ApolloHttpException(
            statusCode = 403,
            headers = emptyList(),
            body = null,
            message = "",
          )
        },
        openDocumentIds = { emptySet() },
      )

    sweeper.sweep()
    sweeper.sweep()
    assertEquals(1, attempts)

    store.records[0] = store.records[0].copy(changeset = byteArrayOf(1, 2))
    sweeper.sweep()
    assertEquals(2, attempts)

    sweeper.resetPermanentFailures()
    sweeper.sweep()
    assertEquals(3, attempts)
  }

  @Test
  fun deleteOnSuccessRemovesOnlyPushedDocuments() = runTest {
    val store = FakeDeltaStore()
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docA", changeset = byteArrayOf(1), createdAt = 1)
    )
    store.records.add(
      DeltaRecord(id = "1:1", documentId = "docB", changeset = byteArrayOf(2), createdAt = 2)
    )
    val sweeper =
      OrphanSweeper(
        store = store,
        pushFn = { documentId, _ -> if (documentId == "docB") throw RuntimeException("network") },
        openDocumentIds = { emptySet() },
      )

    sweeper.sweep(deleteOnSuccess = true)

    assertEquals(emptyList(), store.load("docA").map { it.id })
    assertEquals(listOf("1:1"), store.load("docB").map { it.id })
  }
}
