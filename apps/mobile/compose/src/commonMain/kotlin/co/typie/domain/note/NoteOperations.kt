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
import co.typie.graphql.type.NoteUpdateKind
import co.typie.graphql.type.RemoveNoteEntityInput
import co.typie.graphql.type.UpdateNoteInput
import co.typie.result.Result
import co.typie.result.result

const val DEFAULT_NOTE_COLOR = "gray"

suspend fun createNote(
  siteId: String,
  color: String = DEFAULT_NOTE_COLOR,
  entityId: String? = null,
): Result<NoteCard_note, Nothing> =
  executeSyncedNoteMutation(siteId = siteId, noteId = null, kind = NoteUpdateKind.CREATED) {
    val input =
      CreateNoteInput.Builder()
        .clientId(NoteSync.clientId)
        .content("")
        .color(color)
        .apply {
          if (entityId != null) {
            entityId(entityId)
          } else {
            siteId(siteId)
          }
        }
        .build()
    Apollo.executeMutation(Note_Create_Mutation(input = input)).createNote.noteCard_note
  }

suspend fun updateNoteContent(
  siteId: String,
  noteId: String,
  content: String,
): Result<NoteCard_note, Nothing> =
  executeSyncedNoteMutation(siteId = siteId, noteId = noteId, kind = NoteUpdateKind.UPDATED) {
    Apollo.executeMutation(
        Note_Update_Mutation(
          input =
            UpdateNoteInput.Builder()
              .clientId(NoteSync.clientId)
              .noteId(noteId)
              .content(content)
              .build()
        )
      )
      .updateNote
      .noteCard_note
  }

suspend fun updateNoteColor(
  siteId: String,
  noteId: String,
  color: String,
): Result<NoteCard_note, Nothing> =
  executeSyncedNoteMutation(siteId = siteId, noteId = noteId, kind = NoteUpdateKind.UPDATED) {
    Apollo.executeMutation(
        Note_Update_Mutation(
          input =
            UpdateNoteInput.Builder()
              .clientId(NoteSync.clientId)
              .noteId(noteId)
              .color(color)
              .build()
        )
      )
      .updateNote
      .noteCard_note
  }

suspend fun updateNoteStatus(
  siteId: String,
  noteId: String,
  status: NoteStatus,
): Result<NoteCard_note, Nothing> =
  executeSyncedNoteMutation(siteId = siteId, noteId = noteId, kind = NoteUpdateKind.UPDATED) {
    Apollo.executeMutation(
        Note_Update_Mutation(
          input =
            UpdateNoteInput.Builder()
              .clientId(NoteSync.clientId)
              .noteId(noteId)
              .status(status)
              .build()
        )
      )
      .updateNote
      .noteCard_note
  }

suspend fun deleteNote(siteId: String, noteId: String): Result<String, Nothing> {
  val mutationResult =
    result<String, Nothing> {
      Apollo.executeMutation(
          Note_Delete_Mutation(
            input = DeleteNoteInput.Builder().clientId(NoteSync.clientId).noteId(noteId).build()
          )
        )
        .deleteNote
        .id
    }

  when (mutationResult) {
    is Result.Ok ->
      NoteSync.publish(NoteUpdate(kind = NoteUpdateKind.DELETED, noteId = noteId, siteId = siteId))
    is Result.Exception -> {
      if (mutationResult.isNoteNotFound()) {
        NoteSync.publish(
          NoteUpdate(kind = NoteUpdateKind.DELETED, noteId = noteId, siteId = siteId)
        )
      }
    }

    is Result.Err -> Unit
  }
  return mutationResult
}

suspend fun moveNote(
  note: NoteCard_note,
  lowerOrder: String?,
  upperOrder: String?,
): Result<String, Nothing> {
  val mutationResult =
    result<String, Nothing> {
      Apollo.executeMutation(
          Note_Move_Mutation(
            input =
              MoveNoteInput.Builder()
                .clientId(NoteSync.clientId)
                .noteId(note.id)
                .apply {
                  if (lowerOrder != null) lowerOrder(lowerOrder)
                  if (upperOrder != null) upperOrder(upperOrder)
                }
                .build()
          )
        )
        .moveNote
        .order
    }

  when (mutationResult) {
    is Result.Ok ->
      NoteSync.publish(
        NoteUpdate(kind = NoteUpdateKind.UPDATED, noteId = note.id, siteId = note.site.id)
      )
    is Result.Exception -> {
      if (mutationResult.isNoteNotFound()) {
        NoteSync.publish(
          NoteUpdate(kind = NoteUpdateKind.DELETED, noteId = note.id, siteId = note.site.id)
        )
      }
    }
    is Result.Err -> Unit
  }
  return mutationResult
}

suspend fun addNoteEntity(
  siteId: String,
  noteId: String,
  entityId: String,
): Result<NoteCard_note, Nothing> =
  executeSyncedNoteMutation(siteId = siteId, noteId = noteId, kind = NoteUpdateKind.UPDATED) {
    Apollo.executeMutation(
        Note_AddEntity_Mutation(
          input =
            AddNoteEntityInput.Builder()
              .clientId(NoteSync.clientId)
              .noteId(noteId)
              .entityId(entityId)
              .build()
        )
      )
      .addNoteEntity
      .noteCard_note
  }

suspend fun removeNoteEntity(
  siteId: String,
  noteId: String,
  entityId: String,
): Result<NoteCard_note, Nothing> =
  executeSyncedNoteMutation(siteId = siteId, noteId = noteId, kind = NoteUpdateKind.UPDATED) {
    Apollo.executeMutation(
        Note_RemoveEntity_Mutation(
          input =
            RemoveNoteEntityInput.Builder()
              .clientId(NoteSync.clientId)
              .noteId(noteId)
              .entityId(entityId)
              .build()
        )
      )
      .removeNoteEntity
      .noteCard_note
  }

private suspend fun executeSyncedNoteMutation(
  siteId: String,
  noteId: String?,
  kind: NoteUpdateKind,
  block: suspend () -> NoteCard_note,
): Result<NoteCard_note, Nothing> {
  val mutationResult = result<NoteCard_note, Nothing> { block() }
  when (mutationResult) {
    is Result.Ok -> {
      val note = mutationResult.value
      NoteSync.publish(NoteUpdate(kind = kind, noteId = note.id, siteId = note.site.id))
    }

    is Result.Exception -> {
      if (noteId != null && mutationResult.isNoteNotFound()) {
        NoteSync.publish(
          NoteUpdate(kind = NoteUpdateKind.DELETED, noteId = noteId, siteId = siteId)
        )
      }
    }

    is Result.Err -> Unit
  }
  return mutationResult
}

internal fun NoteEntityPicker_Recent_Query.Data.linkedEntities(): List<NoteEntityPicker_entity> =
  me.recentlyViewedEntities.map { it.noteEntityPicker_entity }.distinctBy { it.id }
