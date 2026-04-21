package co.typie.screen.space.notes

import co.typie.domain.note.notesNote
import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class NotesSceneStateTest {
  @Test
  fun `entering note stays visible until server catches up`() {
    val state = NotesSceneState(NoteStatus.OPEN)
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
    val state = NotesSceneState(NoteStatus.OPEN)
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
    val state = NotesSceneState(NoteStatus.OPEN)
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
    val openScene = NotesSceneState(NoteStatus.OPEN)
    val resolvedScene = NotesSceneState(NoteStatus.RESOLVED)
    val resolvedNote = notesNote(id = "a", order = "100", status = NoteStatus.RESOLVED)

    openScene.markExiting(resolvedNote)
    openScene.finishExiting("a")

    assertEquals(emptyList(), openScene.merge(serverNotes = emptyList()))
    assertEquals(listOf("a"), resolvedScene.merge(serverNotes = listOf(resolvedNote)).map { it.id })
  }

  @Test
  fun `expected destination note starts entering when server catches up`() {
    val state = NotesSceneState(NoteStatus.RESOLVED)
    val note = notesNote(id = "a", order = "100", status = NoteStatus.RESOLVED)

    state.expectEntry(note)

    assertEquals(emptyList(), state.merge(serverNotes = emptyList()))
    assertFalse(state.isEntering("a"))

    state.sync(serverNotes = listOf(note))

    assertTrue(state.isEntering("a"))
    assertEquals(listOf("a"), state.merge(serverNotes = listOf(note)).map { it.id })
  }

  @Test
  fun `remove clears entering and exiting notes`() {
    val state = NotesSceneState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100", status = NoteStatus.OPEN)

    state.markEntering(note)
    state.markExiting(note.copy(status = NoteStatus.RESOLVED))

    state.remove("a")

    assertFalse(state.isEntering("a"))
    assertFalse(state.isExiting("a"))
    assertEquals(emptyList(), state.merge(serverNotes = emptyList()))
  }

  @Test
  fun `scene is settled only after first successful sync`() {
    val state = NotesSceneState(NoteStatus.OPEN)
    val note = notesNote(id = "a", order = "100")

    assertFalse(state.hasSettled)

    state.sync(serverNotes = listOf(note))

    assertTrue(state.hasSettled)
  }
}
