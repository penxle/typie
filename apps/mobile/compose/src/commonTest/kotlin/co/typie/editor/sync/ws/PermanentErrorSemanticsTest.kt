package co.typie.editor.sync.ws

import co.typie.editor.sync.FakeDeltaStore
import co.typie.editor.sync.FakeSyncEditor
import co.typie.editor.sync.SyncEngine
import co.typie.editor.sync.SyncStatus
import co.typie.editor.sync.enc
import co.typie.editor.sync.isPermanentSyncError
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

class PermanentErrorSemanticsTest {
  @Test
  fun forbiddenAndDocumentNotV2ArePermanent() {
    assertTrue(isPermanentSyncError(SyncWsException("forbidden", true)))
    assertTrue(isPermanentSyncError(SyncWsException("document_not_v2", true)))
  }

  @Test
  fun internalAndConnectionLostAreTransient() {
    assertFalse(isPermanentSyncError(SyncWsException("internal", false)))
    assertFalse(isPermanentSyncError(SyncWsException("connection_lost", false)))
  }

  @Test
  fun forbiddenStopsSyncEngineWithNoRetry() = runTest {
    val editor = FakeSyncEditor(listOf(5))
    val store = FakeDeltaStore()
    var pushes = 0
    var clock = 0L
    val engine =
      SyncEngine(
        editor = editor,
        documentId = "doc1",
        initialServerHeads = enc(),
        initialDurableHeads = enc(),
        store = store,
        pushFn = {
          pushes++
          throw SyncWsException("forbidden", true)
        },
        scope = CoroutineScope(coroutineContext),
        isPermanent = ::isPermanentSyncError,
        now = { clock++ },
      )
    runCurrent()
    assertEquals(1, pushes)
    assertEquals(SyncStatus.Error, engine.status.value)

    advanceTimeBy(60_000)
    runCurrent()
    assertEquals(1, pushes)
    engine.stop()
  }
}
