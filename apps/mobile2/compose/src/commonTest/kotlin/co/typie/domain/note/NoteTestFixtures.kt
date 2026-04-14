package co.typie.domain.note

import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteLinkedEntity_entity
import co.typie.graphql.type.NoteStatus
import kotlin.time.Instant

internal fun notesNote(
  id: String,
  order: String = id,
  content: String = "",
  color: String = "gray",
  status: NoteStatus = NoteStatus.OPEN,
  updatedAt: Instant = Instant.parse("2024-01-01T00:00:00Z"),
  entities: List<NoteLinkedEntity_entity> = emptyList(),
) =
  NoteCard_note(
    __typename = "Note",
    id = id,
    content = content,
    order = order,
    color = color,
    status = status,
    updatedAt = updatedAt,
    entities =
      entities.map { NoteCard_note.Entity(__typename = "Entity", noteLinkedEntity_entity = it) },
  )

internal fun notesDocumentEntity(id: String, title: String = "문서") =
  NoteLinkedEntity_entity(
    __typename = "Entity",
    id = id,
    slug = id,
    icon = "file",
    iconColor = "gray",
    node =
      NoteLinkedEntity_entity.Node(
        __typename = "Document",
        onDocument = NoteLinkedEntity_entity.OnDocument(id = "$id-document", title = title),
        onFolder = null,
      ),
  )
