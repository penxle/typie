package co.typie.domain.note

import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NoteListStateTest {
  @Test
  fun `entering note stays visible until server catches up`() {
    val state = NoteListState(NoteStatus.OPEN)
    val created = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)

    state.markEntering(created)
    state.finishEntering("a")

    assertFalse(state.isEntering("a"))
    assertEquals(listOf("a"), state.merge(serverNotes = emptyList()).map { it.id })

    state.sync(serverNotes = listOf(created))

    assertEquals(listOf("a"), state.merge(serverNotes = listOf(created)).map { it.id })
  }

  @Test
  fun `exiting note stays visible until exit finishes then waits for omission`() {
    val state = NoteListState(NoteStatus.OPEN)
    val exiting = notesNote(id = "a", order = "100", status = NoteStatus.RESOLVED)

    state.markExiting(exiting)

    assertTrue(state.isExiting("a"))
    assertEquals(listOf("a"), state.merge(serverNotes = emptyList()).map { it.id })

    state.finishExiting("a")

    assertEquals(emptyList(), state.merge(serverNotes = emptyList()).map { it.id })
    assertTrue(state.isExiting("a"))

    state.sync(serverNotes = emptyList())

    assertFalse(state.isExiting("a"))
  }

  @Test
  fun `finished exit suppresses stale server note until omission`() {
    val state = NoteListState(NoteStatus.OPEN)
    val staleOpen = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)

    state.markExiting(staleOpen.copy(status = NoteStatus.RESOLVED))
    state.finishExiting("a")

    assertEquals(emptyList(), state.merge(serverNotes = listOf(staleOpen)).map { it.id })

    state.sync(serverNotes = listOf(staleOpen))

    assertTrue(state.isExiting("a"))

    state.sync(serverNotes = emptyList())

    assertFalse(state.isExiting("a"))
  }

  @Test
  fun `source scene exit does not hide destination scene note`() {
    val openScene = NoteListState(NoteStatus.OPEN)
    val resolvedScene = NoteListState(NoteStatus.RESOLVED)
    val resolvedNote = notesNote(id = "a", order = "100", status = NoteStatus.RESOLVED)

    openScene.markExiting(resolvedNote)
    openScene.finishExiting("a")

    assertEquals(emptyList(), openScene.merge(serverNotes = emptyList()))
    assertEquals(listOf("a"), resolvedScene.merge(serverNotes = listOf(resolvedNote)).map { it.id })
  }

  @Test
  fun `expected destination note starts entering when server catches up`() {
    val state = NoteListState(NoteStatus.RESOLVED)
    val note = notesNote(id = "a", order = "100", status = NoteStatus.RESOLVED)

    state.expectEntry(note)

    assertEquals(emptyList(), state.merge(serverNotes = emptyList()))
    assertFalse(state.isEntering("a"))

    state.sync(serverNotes = listOf(note))

    assertTrue(state.isEntering("a"))
    assertEquals(listOf("a"), state.merge(serverNotes = listOf(note)).map { it.id })
  }

  @Test
  fun `remote status change keeps the destination style while exiting the source list`() {
    val state = NoteListState(NoteStatus.OPEN)
    val openNote = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)
    val resolvedNote = openNote.copy(status = NoteStatus.RESOLVED)
    state.sync(serverNotes = listOf(openNote))

    state.sync(serverNotes = listOf(resolvedNote))

    assertTrue(state.isExiting(openNote.id))
    assertEquals(NoteStatus.RESOLVED, state.merge(serverNotes = emptyList()).single().status)

    state.finishExiting(openNote.id)
    state.sync(serverNotes = emptyList())

    assertFalse(state.isExiting(openNote.id))
  }

  @Test
  fun `remote status change enters after the destination snapshot arrives`() {
    val state = NoteListState(NoteStatus.RESOLVED)
    val resolvedNote = notesNote(id = "a", order = "100", status = NoteStatus.RESOLVED)
    state.sync(serverNotes = emptyList())

    state.sync(serverNotes = listOf(resolvedNote))

    assertTrue(state.isEntering(resolvedNote.id))
    assertEquals(listOf(resolvedNote.id), state.merge(listOf(resolvedNote)).map { it.id })
  }

  @Test
  fun `remove clears entering and exiting notes`() {
    val state = NoteListState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)

    state.markEntering(note)
    state.markExiting(note.copy(status = NoteStatus.RESOLVED))

    state.remove("a")

    assertFalse(state.isEntering("a"))
    assertFalse(state.isExiting("a"))
    assertEquals(emptyList(), state.merge(serverNotes = emptyList()))
  }

  @Test
  fun `terminal deletion starts exit once`() {
    val state = NoteListState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)

    state.markExiting(note)
    state.finishExiting(note.id)
    state.markExiting(note)

    assertTrue(state.isExiting(note.id))
    assertFalse(state.isExitVisible(note.id))
  }

  @Test
  fun `terminal deletion animates an optimistic entering note`() {
    val state = NoteListState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)

    state.markEntering(note)
    state.markDeleted(note.id)

    assertFalse(state.isEntering(note.id))
    assertTrue(state.isExiting(note.id))
    assertEquals(listOf("a"), state.merge(serverNotes = emptyList()).map { it.id })
  }

  @Test
  fun `scene is settled only after first successful sync`() {
    val state = NoteListState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100")

    assertFalse(state.hasSettled)

    state.sync(serverNotes = listOf(note))

    assertTrue(state.hasSettled)
  }

  @Test
  fun `first successful sync does not animate existing notes`() {
    val state = NoteListState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100")

    state.sync(serverNotes = listOf(note))

    assertFalse(state.isEntering(note.id))
    assertFalse(state.isExiting(note.id))
  }

  @Test
  fun `later remote addition enters and remote omission exits`() {
    val state = NoteListState(NoteStatus.OPEN)
    val existing = notesNote(id = "a", order = "100")
    val added = notesNote(id = "b", order = "200")

    state.sync(serverNotes = listOf(existing))
    state.sync(serverNotes = listOf(existing, added))

    assertTrue(state.isEntering(added.id))

    state.finishEntering(added.id)
    state.sync(serverNotes = listOf(added))

    assertTrue(state.isExiting(existing.id))
    assertEquals(
      listOf(existing.id, added.id),
      state.merge(serverNotes = listOf(added)).map { it.id },
    )
  }
}
