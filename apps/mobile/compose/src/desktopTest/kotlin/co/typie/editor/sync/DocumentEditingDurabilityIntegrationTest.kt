package co.typie.editor.sync

import app.cash.sqldelight.driver.jdbc.sqlite.JdbcSqliteDriver
import co.typie.editor.EditingCheckpointResult
import co.typie.editor.Editor
import co.typie.editor.FakeFfiEditor
import co.typie.editor.ffi.EditorEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StateField
import co.typie.editor.ffi.SystemEvent
import co.typie.sync.db.SyncDatabase
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlinx.coroutines.async
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

class DocumentEditingDurabilityIntegrationTest {
  @Test
  fun capturedEditIsPushedByOrphanSweeperAfterSessionStops() = runTest {
    val driver = JdbcSqliteDriver(JdbcSqliteDriver.IN_MEMORY)
    SyncDatabase.Schema.create(driver)
    val store = SqlDeltaStore(SyncDatabase(driver))
    val syncEditor = FakeSyncEditor()
    val editor =
      Editor(
        FakeFfiEditor(
          onTick = {
            syncEditor.known.add(1)
            listOf(EditorEvent.StateChanged(listOf(StateField.Doc)))
          }
        ),
        this,
        StandardTestDispatcher(testScheduler),
      )
    val session =
      createTestDocumentEditingSession(
        editor = editor,
        scope = this,
        syncEditor = syncEditor,
        store = store,
        pushFn = { throw IllegalStateException("offline") },
      )
    ActiveDocumentEditingSessions.register(session)
    try {
      val edit = session.submit { sessionEditor, context ->
        async(context) { sessionEditor.await { enqueue(Message.System(SystemEvent.Initialize)) } }
      }
      assertNotNull(edit)

      assertEquals(EditingCheckpointResult.Protected, session.beginStop().awaitCheckpoint())
      assertEquals(listOf("1"), store.load("doc").map { it.id })
      session.stop()
      ActiveDocumentEditingSessions.unregister(session)

      var pushedDocumentId: String? = null
      var pushedChangesets: ByteArray? = null
      OrphanSweeper(
          store = store,
          pushFn = { documentId, changesets ->
            pushedDocumentId = documentId
            pushedChangesets = changesets
          },
          openDocumentIds = { ActiveDocumentEditingSessions.openDocumentIds() },
        )
        .sweep()

      assertEquals("doc", pushedDocumentId)
      assertContentEquals(enc(1), pushedChangesets)
    } finally {
      session.stop()
      ActiveDocumentEditingSessions.unregister(session)
    }
  }
}
