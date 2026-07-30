package co.typie.domain.note

import co.typie.graphql.type.NoteStatus
import kotlin.test.Test
import kotlin.test.assertEquals

class NoteHelpersTest {
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
}
