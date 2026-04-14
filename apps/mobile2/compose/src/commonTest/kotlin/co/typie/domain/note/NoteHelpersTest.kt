package co.typie.domain.note

import co.typie.graphql.type.NoteStatus
import co.typie.screen.space.notes.emptyMessage
import co.typie.screen.space.notes.filterLabel
import co.typie.screen.space.notes.toggled
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class NoteHelpersTest {
  @Test
  fun `resolveMovedNoteOrders returns adjacent bounds for middle note`() {
    val notes =
      listOf(
        notesNote(id = "a", order = "100"),
        notesNote(id = "b", order = "200"),
        notesNote(id = "c", order = "300"),
      )

    assertEquals(
      NoteMoveOrders(lowerOrder = "100", upperOrder = "300"),
      resolveMovedNoteOrders(notes, movedNoteId = "b"),
    )
  }

  @Test
  fun `resolveMovedNoteOrders returns null bound at list edges`() {
    val notes =
      listOf(
        notesNote(id = "a", order = "100"),
        notesNote(id = "b", order = "200"),
        notesNote(id = "c", order = "300"),
      )

    assertEquals(
      NoteMoveOrders(lowerOrder = null, upperOrder = "200"),
      resolveMovedNoteOrders(notes, movedNoteId = "a"),
    )
    assertEquals(
      NoteMoveOrders(lowerOrder = "200", upperOrder = null),
      resolveMovedNoteOrders(notes, movedNoteId = "c"),
    )
  }

  @Test
  fun `resolveMovedNoteOrders returns null when note is missing`() {
    val notes = listOf(notesNote(id = "a", order = "100"))

    assertNull(resolveMovedNoteOrders(notes, movedNoteId = "missing"))
  }

  @Test
  fun `status labels and empty messages follow screen copy`() {
    assertEquals("진행 중", NoteStatus.OPEN.filterLabel())
    assertEquals("진행 중 노트가 없어요", NoteStatus.OPEN.emptyMessage())
    assertEquals(NoteStatus.RESOLVED, NoteStatus.OPEN.toggled())

    assertEquals("완료됨", NoteStatus.RESOLVED.filterLabel())
    assertEquals("완료된 노트가 없어요", NoteStatus.RESOLVED.emptyMessage())
    assertEquals(NoteStatus.OPEN, NoteStatus.RESOLVED.toggled())
  }

  @Test
  fun `buildCollapsedMeta keeps first entity and reports overflow`() {
    val entities =
      listOf(
        notesDocumentEntity(id = "1", title = "문서 1"),
        notesDocumentEntity(id = "2", title = "문서 2"),
        notesDocumentEntity(id = "3", title = "문서 3"),
      )

    val meta = buildCollapsedMeta(entities)

    assertEquals(listOf("1"), meta.visibleEntities.map { it.id })
    assertEquals(2, meta.overflowCount)
  }

  @Test
  fun `displayOrderedNotes falls back when reorder keys are stale`() {
    val notes = listOf(notesNote(id = "a", order = "100"), notesNote(id = "b", order = "200"))

    assertEquals(
      notes,
      displayOrderedNotes(notes, orderedKeys = listOf("placeholder-1", "placeholder-2")),
    )
  }

  @Test
  fun `displayOrderedNotes uses displayed order when complete`() {
    val notes = listOf(notesNote(id = "a", order = "100"), notesNote(id = "b", order = "200"))

    assertEquals(
      listOf("b", "a"),
      displayOrderedNotes(notes, orderedKeys = listOf("b", "a")).map { it.id },
    )
  }
}
