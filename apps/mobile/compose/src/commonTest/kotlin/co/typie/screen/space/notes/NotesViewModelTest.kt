package co.typie.screen.space.notes

import co.typie.domain.note.NoteSync
import co.typie.domain.note.NoteUpdate
import co.typie.domain.note.notesNote
import co.typie.graphql.QueryState
import co.typie.graphql.type.NoteStatus
import co.typie.graphql.type.NoteUpdateKind
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotSame
import kotlin.test.assertSame

class NotesViewModelTest {
  @Test
  fun `changing site discards the previous site scenes`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    val siteAResolved = NotesSceneKey(siteId = "site-a", status = NoteStatus.RESOLVED)
    val noteA = notesNote(id = "a", status = NoteStatus.OPEN)
    val noteB = notesNote(id = "b", status = NoteStatus.OPEN)
    val resolvedA = notesNote(id = "resolved-a", status = NoteStatus.RESOLVED)

    cache.activateSite("site-a")
    cache.commitSuccess(siteId = "site-a", notes = listOf(noteA, resolvedA))
    assertEquals(
      listOf("resolved-a"),
      cache
        .notes(
          key = siteAResolved,
          querySiteId = null,
          queryState = QueryState.Loading,
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )

    cache.activateSite("site-b")
    cache.commitSuccess(siteId = "site-b", notes = listOf(noteB))

    assertEquals(
      emptyList(),
      cache
        .notes(
          key = siteAOpen,
          querySiteId = null,
          queryState = QueryState.Loading,
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )
    assertEquals(
      listOf("b"),
      cache
        .notes(
          key = siteBOpen,
          querySiteId = null,
          queryState = QueryState.Loading,
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )
  }

  @Test
  fun `active query data is used only for the matching site and status`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    val oldNote = notesNote(id = "old", status = NoteStatus.OPEN)
    val activeNote = notesNote(id = "active", status = NoteStatus.OPEN)

    cache.commitSuccess(siteId = "site-a", notes = listOf(oldNote))

    assertEquals(
      listOf("active"),
      cache
        .notes(
          key = siteAOpen,
          querySiteId = siteAOpen.siteId,
          queryState = QueryState.Success(listOf(activeNote)),
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )
    assertEquals(
      emptyList(),
      cache
        .notes(
          key = siteBOpen,
          querySiteId = siteAOpen.siteId,
          queryState = QueryState.Success(listOf(activeNote)),
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )
  }

  @Test
  fun `active query data is ignored when the response key does not match the scene key`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    val staleNote = notesNote(id = "stale", status = NoteStatus.OPEN)

    assertEquals(
      emptyList(),
      cache
        .notes(
          key = siteBOpen,
          querySiteId = siteAOpen.siteId,
          queryState = QueryState.Success(listOf(staleNote)),
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )
  }

  @Test
  fun `late success from the previous site cannot repopulate its cache`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    cache.activateSite("site-b")

    cache.commitSuccess(
      siteId = siteAOpen.siteId,
      notes = listOf(notesNote(id = "late", siteId = "site-a", status = NoteStatus.OPEN)),
    )

    assertEquals(
      emptyList(),
      cache.notes(
        key = siteAOpen,
        querySiteId = null,
        queryState = QueryState.Loading,
        placeholderNotes = emptyList(),
      ),
    )
    assertEquals(
      QueryState.Loading,
      cache.queryState(
        key = siteBOpen,
        querySiteId = siteAOpen.siteId,
        queryState = QueryState.Success(emptyList()),
      ),
    )
  }

  @Test
  fun `list animation state is scoped by site and status`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    val noteA = notesNote(id = "a", status = NoteStatus.OPEN)

    val siteAOpenState = cache.listState(siteAOpen)
    siteAOpenState.markEntering(noteA)

    assertSame(siteAOpenState, cache.listState(siteAOpen))
    assertNotSame(siteAOpenState, cache.listState(siteBOpen))
    assertEquals(listOf("a"), siteAOpenState.merge(serverNotes = emptyList()).map { it.id })
    assertFalse(cache.listState(siteBOpen).isEntering("a"))
  }

  @Test
  fun `error state is exposed only for the active site and status`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    val error = QueryState.Error(Exception("failed"))

    assertSame(
      error,
      cache.queryState(key = siteAOpen, querySiteId = siteAOpen.siteId, queryState = error),
    )
    assertEquals(
      QueryState.Loading,
      cache.queryState(key = siteBOpen, querySiteId = siteAOpen.siteId, queryState = error),
    )
  }

  @Test
  fun `terminal deletion starts exit only in scenes that contain the note`() {
    val cache = NotesSceneCache()
    val openKey = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val resolvedKey = NotesSceneKey(siteId = "site-a", status = NoteStatus.RESOLVED)
    val note = notesNote(id = "a", status = NoteStatus.OPEN)

    cache.commitSuccess(siteId = openKey.siteId, notes = listOf(note))
    cache.convergeDeletedNote(
      noteId = note.id,
      querySiteId = openKey.siteId,
      queryNotes = listOf(note),
    )

    assertEquals(true, cache.listState(openKey).isExiting(note.id))
    assertFalse(cache.listState(resolvedKey).isExiting(note.id))
  }

  @Test
  fun `terminal tombstone filters a stale successful query snapshot`() {
    val cache = NotesSceneCache()
    val key = NotesSceneKey(siteId = "terminal-cache-site", status = NoteStatus.OPEN)
    val note = notesNote(id = "terminal-cache-note", status = NoteStatus.OPEN)
    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = note.id, siteId = key.siteId))

    cache.commitSuccess(siteId = key.siteId, notes = listOf(note))

    assertEquals(
      emptyList(),
      cache.notes(
        key = key,
        querySiteId = key.siteId,
        queryState = QueryState.Success(listOf(note)),
        placeholderNotes = emptyList(),
      ),
    )

    NoteSync.publish(NoteUpdate(NoteUpdateKind.CREATED, noteId = note.id, siteId = key.siteId))
  }

  @Test
  fun `one authoritative snapshot drives both sides of a remote status change`() {
    val cache = NotesSceneCache()
    val siteId = "site-a"
    val openKey = NotesSceneKey(siteId = siteId, status = NoteStatus.OPEN)
    val resolvedKey = NotesSceneKey(siteId = siteId, status = NoteStatus.RESOLVED)
    val openNote = notesNote(id = "a", siteId = siteId, status = NoteStatus.OPEN)
    val resolvedNote = openNote.copy(status = NoteStatus.RESOLVED)

    cache.commitSuccess(siteId = siteId, notes = listOf(openNote))
    cache.commitSuccess(siteId = siteId, notes = listOf(resolvedNote))

    assertEquals(true, cache.listState(openKey).isExiting(openNote.id))
    assertEquals(
      NoteStatus.RESOLVED,
      cache.listState(openKey).merge(serverNotes = emptyList()).single().status,
    )
    assertEquals(true, cache.listState(resolvedKey).isEntering(openNote.id))
  }
}
