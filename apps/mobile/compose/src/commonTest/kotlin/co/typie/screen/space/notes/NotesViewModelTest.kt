package co.typie.screen.space.notes

import co.typie.domain.note.notesNote
import co.typie.graphql.QueryState
import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotSame
import kotlin.test.assertSame

class NotesViewModelTest {
  @Test
  fun `settled notes are scoped by site and status`() {
    val cache = NotesSceneCache()
    val siteAOpen = NotesSceneKey(siteId = "site-a", status = NoteStatus.OPEN)
    val siteBOpen = NotesSceneKey(siteId = "site-b", status = NoteStatus.OPEN)
    val siteAResolved = NotesSceneKey(siteId = "site-a", status = NoteStatus.RESOLVED)
    val noteA = notesNote(id = "a", status = NoteStatus.OPEN)
    val noteB = notesNote(id = "b", status = NoteStatus.OPEN)
    val resolvedA = notesNote(id = "resolved-a", status = NoteStatus.RESOLVED)

    cache.commitSuccess(siteAOpen, listOf(noteA))
    cache.commitSuccess(siteBOpen, listOf(noteB))
    cache.commitSuccess(siteAResolved, listOf(resolvedA))

    assertEquals(
      listOf("a"),
      cache
        .notes(
          key = siteAOpen,
          queryKey = null,
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
          queryKey = null,
          queryState = QueryState.Loading,
          placeholderNotes = emptyList(),
        )
        .map { it.id },
    )
    assertEquals(
      listOf("resolved-a"),
      cache
        .notes(
          key = siteAResolved,
          queryKey = null,
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

    cache.commitSuccess(siteAOpen, listOf(oldNote))

    assertEquals(
      listOf("active"),
      cache
        .notes(
          key = siteAOpen,
          queryKey = siteAOpen,
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
          queryKey = siteAOpen,
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
          queryKey = siteAOpen,
          queryState = QueryState.Success(listOf(staleNote)),
          placeholderNotes = emptyList(),
        )
        .map { it.id },
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
      cache.queryState(
        key = siteAOpen,
        activeKey = siteAOpen,
        queryKey = siteAOpen,
        queryState = error,
      ),
    )
    assertEquals(
      QueryState.Loading,
      cache.queryState(
        key = siteBOpen,
        activeKey = siteAOpen,
        queryKey = siteAOpen,
        queryState = error,
      ),
    )
  }
}
