package co.typie.domain.note

import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.time.Instant
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class NoteEditStateTest {
  @Test
  fun `open tracks expanded note`() = runTest {
    val state = NoteEditState(scope = this, contentDebounceMillis = 300L)
    val note = notesNote(id = "existing")

    state.open(note = note)

    assertEquals("existing", state.expandedNoteId)
  }

  @Test
  fun `debounced content save runs after delay`() = runTest {
    val state = NoteEditState(scope = this, contentDebounceMillis = 300L)
    val saved = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note"))

    state.updateContent(noteId = "note", value = "updated") { noteId, content ->
      delay(1)
      saved += noteId to content
      true
    }

    advanceTimeBy(299)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertEquals(listOf("note" to "updated"), saved)
  }

  @Test
  fun `debounced color save runs after delay`() = runTest {
    val state =
      NoteEditState(scope = this, contentDebounceMillis = 300L, colorDebounceMillis = 180L)
    val saved = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note", color = "gray"))

    state.updateColor(noteId = "note", value = "red") { noteId, color ->
      delay(1)
      saved += noteId to color
      true
    }

    advanceTimeBy(179)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertTrue(saved.isEmpty())

    advanceTimeBy(1)
    runCurrent()
    assertEquals(listOf("note" to "red"), saved)
  }

  @Test
  fun `overlay preserves latest server snapshot fields while applying local drafts`() = runTest {
    val state = NoteEditState(scope = this, contentDebounceMillis = 180L)
    val linkedEntity = notesDocumentEntity(id = "entity-1")
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(noteId = "note", value = "draft content") { _, _ -> true }
    state.updateColor(noteId = "note", value = "red") { _, _ -> true }

    val overlaid =
      state.commitServerSnapshot(
        notesNote(
          id = "note",
          content = "server snapshot",
          color = "gray",
          status = NoteStatus.RESOLVED,
          updatedAt = Instant.parse("2024-01-02T00:00:00Z"),
          entities = listOf(linkedEntity),
        )
      )

    assertEquals("draft content", overlaid.content)
    assertEquals("red", overlaid.color)
    assertEquals(NoteStatus.RESOLVED, overlaid.status)
    assertEquals(Instant.parse("2024-01-02T00:00:00Z"), overlaid.updatedAt)
    assertEquals(listOf(linkedEntity.id), overlaid.entities.map { it.noteLinkedEntity_entity.id })
    assertEquals(
      listOf(linkedEntity.id),
      state.overlay(notesNote(id = "note", content = "server", color = "gray")).entities.map {
        it.noteLinkedEntity_entity.id
      },
    )
  }

  @Test
  fun `unrelated note snapshot does not clear active draft`() = runTest {
    val state = NoteEditState(scope = this, contentDebounceMillis = 180L)
    state.open(note = notesNote(id = "open", color = "gray"))
    state.updateColor(noteId = "open", value = "red") { _, _ -> true }

    state.commitServerSnapshot(
      notesNote(id = "other", color = "blue", status = NoteStatus.RESOLVED)
    )

    assertEquals("red", state.overlay(notesNote(id = "open", color = "gray")).color)
    assertTrue(state.hasPendingColor("open"))
  }

  @Test
  fun `flush persists both content and color before collapse`() = runTest {
    val state =
      NoteEditState(scope = this, contentDebounceMillis = 300L, colorDebounceMillis = 180L)
    val contentSaves = mutableListOf<Pair<String, String>>()
    val colorSaves = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))

    state.updateContent(noteId = "note", value = "local content") { noteId, content ->
      contentSaves += noteId to content
      true
    }
    state.updateColor(noteId = "note", value = "red") { noteId, color ->
      colorSaves += noteId to color
      true
    }

    val collapsed =
      state.collapse(
        saveColor = { noteId, color ->
          colorSaves += noteId to color
          true
        },
        saveContent = { noteId, content ->
          contentSaves += noteId to content
          true
        },
      )

    assertTrue(collapsed)
    assertEquals(listOf("note" to "local content"), contentSaves)
    assertEquals(listOf("note" to "red"), colorSaves)
    assertEquals(null, state.expandedNoteId)
  }

  @Test
  fun `dispose saves pending drafts and keeps expanded note`() = runTest {
    val state =
      NoteEditState(scope = this, contentDebounceMillis = 300L, colorDebounceMillis = 180L)
    val contentSaves = mutableListOf<Pair<String, String>>()
    val colorSaves = mutableListOf<Pair<String, String>>()
    state.open(note = notesNote(id = "note", content = "server", color = "gray"))
    state.updateContent(noteId = "note", value = "local content") { noteId, content ->
      contentSaves += noteId to content
      true
    }
    state.updateColor(noteId = "note", value = "red") { noteId, color ->
      colorSaves += noteId to color
      true
    }

    state.dispose(
      savePendingContent = { noteId, content -> contentSaves += noteId to content },
      savePendingColor = { noteId, color -> colorSaves += noteId to color },
    )

    assertEquals(listOf("note" to "local content"), contentSaves)
    assertEquals(listOf("note" to "red"), colorSaves)
    assertEquals("note", state.expandedNoteId)
    assertEquals(
      notesNote(id = "note", content = "local content", color = "red"),
      state.overlay(notesNote(id = "note", content = "server", color = "gray")),
    )
  }

  @Test
  fun `older content save completion does not clear newer draft before snapshot`() = runTest {
    val state = NoteEditState(scope = this, contentDebounceMillis = 10L)
    val firstSaveStarted = CompletableDeferred<Unit>()
    val finishFirstSave = CompletableDeferred<Unit>()
    val saved = mutableListOf<String>()
    state.open(note = notesNote(id = "note", content = "server"))

    state.updateContent(noteId = "note", value = "draft A") { _, content ->
      saved += content
      firstSaveStarted.complete(Unit)
      finishFirstSave.await()
      true
    }

    advanceTimeBy(10)
    runCurrent()
    firstSaveStarted.await()

    state.updateContent(noteId = "note", value = "draft B") { _, content ->
      saved += content
      true
    }
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
    assertTrue(state.isDirty("note"))

    finishFirstSave.complete(Unit)
    runCurrent()

    assertEquals(listOf("draft A"), saved)
    assertEquals("draft B", state.overlay(notesNote(id = "note", content = "server")).content)
    assertTrue(state.isDirty("note"))

    val overlaid = state.commitServerSnapshot(notesNote(id = "note", content = "draft A"))
    assertEquals("draft B", overlaid.content)
    assertTrue(state.isDirty("note"))
  }
}
