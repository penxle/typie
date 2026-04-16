package co.typie.domain.note

import co.typie.graphql.Apollo
import co.typie.graphql.NoteEntityPicker_Recent_Query
import co.typie.graphql.Note_AddEntity_Mutation
import co.typie.graphql.Note_Create_Mutation
import co.typie.graphql.Note_Delete_Mutation
import co.typie.graphql.Note_Move_Mutation
import co.typie.graphql.Note_RemoveEntity_Mutation
import co.typie.graphql.Note_Update_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.fragment.NoteCard_note
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.graphql.type.AddNoteEntityInput
import co.typie.graphql.type.CreateNoteInput
import co.typie.graphql.type.DeleteNoteInput
import co.typie.graphql.type.MoveNoteInput
import co.typie.graphql.type.NoteStatus
import co.typie.graphql.type.RemoveNoteEntityInput
import co.typie.graphql.type.UpdateNoteInput
import co.typie.result.Result
import co.typie.result.result
import co.typie.storage.Preference

const val DEFAULT_NOTE_COLOR = "gray"

suspend fun createNote(color: String = DEFAULT_NOTE_COLOR): Result<NoteCard_note, Nothing> =
  result {
    val siteId = Preference.siteId ?: error("missing siteId")
    Apollo.executeMutation(
        Note_Create_Mutation(
          input = CreateNoteInput.Builder().siteId(siteId).content("").color(color).build()
        )
      )
      .createNote
      .noteCard_note
  }

suspend fun updateNoteContent(noteId: String, content: String): Result<NoteCard_note, Nothing> =
  result {
    Apollo.executeMutation(
        Note_Update_Mutation(
          input = UpdateNoteInput.Builder().noteId(noteId).content(content).build()
        )
      )
      .updateNote
      .noteCard_note
  }

suspend fun updateNoteColor(noteId: String, color: String): Result<NoteCard_note, Nothing> =
  result {
    Apollo.executeMutation(
        Note_Update_Mutation(input = UpdateNoteInput.Builder().noteId(noteId).color(color).build())
      )
      .updateNote
      .noteCard_note
  }

suspend fun updateNoteStatus(noteId: String, status: NoteStatus): Result<NoteCard_note, Nothing> =
  result {
    Apollo.executeMutation(
        Note_Update_Mutation(
          input = UpdateNoteInput.Builder().noteId(noteId).status(status).build()
        )
      )
      .updateNote
      .noteCard_note
  }

suspend fun deleteNote(noteId: String): Result<Unit, Nothing> = result {
  Apollo.executeMutation(Note_Delete_Mutation(input = DeleteNoteInput(noteId = noteId)))
}

suspend fun moveNote(
  noteId: String,
  lowerOrder: String?,
  upperOrder: String?,
): Result<Unit, Nothing> = result {
  Apollo.executeMutation(
    Note_Move_Mutation(
      input =
        MoveNoteInput.Builder()
          .noteId(noteId)
          .apply {
            if (lowerOrder != null) lowerOrder(lowerOrder)
            if (upperOrder != null) upperOrder(upperOrder)
          }
          .build()
    )
  )
}

suspend fun addNoteEntity(noteId: String, entityId: String): Result<NoteCard_note, Nothing> =
  result {
    Apollo.executeMutation(
        Note_AddEntity_Mutation(input = AddNoteEntityInput(noteId = noteId, entityId = entityId))
      )
      .addNoteEntity
      .noteCard_note
  }

suspend fun removeNoteEntity(noteId: String, entityId: String): Result<NoteCard_note, Nothing> =
  result {
    Apollo.executeMutation(
        Note_RemoveEntity_Mutation(
          input = RemoveNoteEntityInput(noteId = noteId, entityId = entityId)
        )
      )
      .removeNoteEntity
      .noteCard_note
  }

internal fun NoteEntityPicker_Recent_Query.Data.linkedEntities(): List<NoteEntityPicker_entity> =
  me.recentlyViewedEntities.map { it.noteEntityPicker_entity }.distinctBy { it.id }
