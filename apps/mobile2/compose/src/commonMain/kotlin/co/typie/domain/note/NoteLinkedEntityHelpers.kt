package co.typie.domain.note

import co.typie.domain.entity.parentFolderMeta
import co.typie.graphql.fragment.EntityParentMeta_folder
import co.typie.graphql.fragment.EntityRow_entity
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.graphql.fragment.NoteLinkedEntity_entity

internal fun NoteCard_note.linkedEntities(): List<NoteLinkedEntity_entity> = entities.map {
  it.noteLinkedEntity_entity
}

internal val NoteLinkedEntity_entity.entity: EntityRow_entity
  get() = entityRow_entity

internal val NoteLinkedEntity_entity.id: String
  get() = entity.id

internal val NoteEntityPicker_entity.entity: EntityRow_entity
  get() = entityRow_entity

internal val NoteEntityPicker_entity.id: String
  get() = entity.id

internal fun NoteEntityPicker_entity.parentFolder(): EntityParentMeta_folder? {
  return entityRowParent_entity.parentFolderMeta()
}
