package co.typie.domain.note

import co.typie.graphql.TypieError
import co.typie.graphql.type.NoteUpdateKind
import co.typie.result.Result
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.test.runTest

class NoteSyncTest {
  @Test
  fun `only canonical note not found errors are terminal`() {
    assertTrue(Result.Exception(TypieError("not_found", null)).isNoteNotFound())
    assertFalse(Result.Exception(TypieError("forbidden", null)).isNoteNotFound())
    assertFalse(Result.Exception(Exception("offline")).isNoteNotFound())
  }

  @Test
  fun `terminal deletions are site scoped and replayed to later listeners`() = runTest {
    val noteId = "terminal-note"
    val siteA = "terminal-site-a"
    val siteB = "terminal-site-b"

    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = noteId, siteId = siteA))
    val siteADeletes = mutableListOf<String>()
    val siteBDeletes = mutableListOf<String>()
    val unregisterSiteA = NoteSync.onTerminalDelete(siteA) { siteADeletes += it }
    val unregisterSiteB = NoteSync.onTerminalDelete(siteB) { siteBDeletes += it }

    assertTrue(NoteSync.isTerminallyDeleted(siteA, noteId))
    assertFalse(NoteSync.isTerminallyDeleted(siteB, noteId))
    assertEquals(listOf(noteId), siteADeletes)
    assertEquals(emptyList(), siteBDeletes)

    NoteSync.publish(NoteUpdate(NoteUpdateKind.CREATED, noteId = noteId, siteId = siteA))

    assertFalse(NoteSync.isTerminallyDeleted(siteA, noteId))
    unregisterSiteA()
    unregisterSiteB()
  }

  @Test
  fun `terminal listener emits only newly added ids and emits a recreated id again`() = runTest {
    val siteId = "incremental-terminal-site"
    val noteA = "incremental-terminal-a"
    val noteB = "incremental-terminal-b"
    val terminalDeletes = mutableListOf<String>()
    val unregister = NoteSync.onTerminalDelete(siteId) { terminalDeletes += it }

    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = noteA, siteId = siteId))
    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = noteA, siteId = siteId))
    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = noteB, siteId = siteId))
    NoteSync.publish(NoteUpdate(NoteUpdateKind.CREATED, noteId = noteA, siteId = siteId))
    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = noteA, siteId = siteId))

    assertEquals(listOf(noteA, noteB, noteA), terminalDeletes)
    unregister()
  }
}
