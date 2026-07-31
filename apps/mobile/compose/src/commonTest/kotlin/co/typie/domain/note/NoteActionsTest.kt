package co.typie.domain.note

import co.typie.graphql.TypieError
import co.typie.graphql.type.NoteStatus
import co.typie.graphql.type.NoteUpdateKind
import co.typie.result.Result
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest

class NoteActionsTest {
  @Test
  fun `terminal state wins over a late successful mutation result`() = runTest {
    val siteId = "actions-terminal-site"
    val note = notesNote(id = "actions-terminal-note", siteId = siteId)
    val converged = mutableListOf<String>()
    val actions = NoteActions()
    actions.activate(
      siteId = siteId,
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = { converged += it },
    )
    NoteSync.publish(NoteUpdate(NoteUpdateKind.DELETED, noteId = note.id, siteId = siteId))

    val outcome = actions.update(actions.captureRequest()!!, note.id) { Result.Ok(note) }

    assertEquals(NoteActionOutcome.Terminal, outcome)
    assertEquals(listOf(note.id), converged)
  }

  @Test
  fun `terminal listener follows the active identity and stops after disposal`() = runTest {
    val oldSiteId = "actions-listener-old-site"
    val newSiteId = "actions-listener-new-site"
    val actions = NoteActions()
    val converged = mutableListOf<String>()
    actions.activate(
      siteId = oldSiteId,
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = { converged += "old:$it" },
    )
    actions.activate(
      siteId = newSiteId,
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = { converged += "new:$it" },
    )

    NoteSync.publish(
      NoteUpdate(NoteUpdateKind.DELETED, noteId = "old-listener-note", siteId = oldSiteId)
    )
    NoteSync.publish(
      NoteUpdate(NoteUpdateKind.DELETED, noteId = "new-listener-note", siteId = newSiteId)
    )
    actions.dispose()
    NoteSync.publish(
      NoteUpdate(NoteUpdateKind.DELETED, noteId = "disposed-listener-note", siteId = newSiteId)
    )

    assertEquals(listOf("new:new-listener-note"), converged)
  }

  @Test
  fun `canonical not found converges while ordinary failures remain failures`() = runTest {
    val siteId = "actions-failure-site"
    val noteId = "actions-failure-note"
    val ordinaryFailureNoteId = "actions-ordinary-failure-note"
    val converged = mutableListOf<String>()
    val actions = NoteActions()
    actions.activate(
      siteId = siteId,
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = { converged += it },
    )

    assertEquals(
      NoteActionOutcome.Terminal,
      actions.update(actions.captureRequest()!!, noteId) {
        Result.Exception(TypieError("not_found", null))
      },
    )

    val failure =
      assertIs<NoteActionOutcome.Failure>(
        actions.update(actions.captureRequest()!!, ordinaryFailureNoteId) {
          Result.Exception(Exception("offline"))
        }
      )
    assertEquals("offline", failure.error.message)
    assertEquals(listOf(noteId), converged)
  }

  @Test
  fun `detached save not found records terminal convergence`() {
    val siteId = "detached-save-not-found-site"
    val noteId = "detached-save-not-found-note"

    val outcome =
      Result.Exception(TypieError("not_found", null))
        .toNoteSaveOutcome(siteId = siteId, noteId = noteId)

    assertEquals(NoteSaveOutcome.Saved, outcome)
    assertTrue(NoteSync.isTerminallyDeleted(siteId = siteId, noteId = noteId))
  }

  @Test
  fun `completion from a replaced surface is ignored`() = runTest {
    val actions = NoteActions()
    val oldEditState = createNoteEditState()
    val newEditState = createNoteEditState()
    val oldNote = notesNote(id = "old-note", siteId = "site")
    val mutationStarted = CompletableDeferred<Unit>()
    val finishMutation = CompletableDeferred<Unit>()
    val converged = mutableListOf<String>()
    actions.activate("site", "old-document", oldEditState) { converged += it }
    val request = actions.captureRequest()!!

    var outcome: NoteActionOutcome<*>? = null
    val mutation = launch {
      outcome =
        actions.update(request, oldNote.id) {
          mutationStarted.complete(Unit)
          finishMutation.await()
          Result.Exception(Exception("late failure"))
        }
    }
    mutationStarted.await()

    actions.activate("site", "new-document", newEditState) { converged += it }
    var staleMutationInvoked = false
    assertEquals(
      NoteActionOutcome.Superseded,
      actions.update(request, oldNote.id) {
        staleMutationInvoked = true
        Result.Ok(oldNote)
      },
    )
    finishMutation.complete(Unit)
    mutation.join()

    assertEquals(NoteActionOutcome.Superseded, outcome)
    assertEquals(false, staleMutationInvoked)
    assertEquals(emptyList(), converged)
  }

  @Test
  fun `a replaced create cannot clear the new surface request`() = runTest {
    val actions = NoteActions()
    val oldCreateStarted = CompletableDeferred<Unit>()
    val finishOldCreate = CompletableDeferred<Unit>()
    val newCreateStarted = CompletableDeferred<Unit>()
    val finishNewCreate = CompletableDeferred<Unit>()
    val oldNote = notesNote(id = "old-note", siteId = "old-site")
    val newNote = notesNote(id = "new-note", siteId = "new-site")
    actions.activate(
      siteId = "old-site",
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = {},
    )
    val oldRequest = actions.captureRequest()!!

    val oldCreate = async {
      actions.create(oldRequest) {
        oldCreateStarted.complete(Unit)
        finishOldCreate.await()
        Result.Ok(oldNote)
      }
    }
    oldCreateStarted.await()

    actions.activate(
      siteId = "new-site",
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = {},
    )
    val newRequest = actions.captureRequest()!!
    val newCreate = async {
      actions.create(newRequest) {
        newCreateStarted.complete(Unit)
        finishNewCreate.await()
        Result.Ok(newNote)
      }
    }
    newCreateStarted.await()
    finishOldCreate.complete(Unit)

    assertEquals(NoteActionOutcome.Superseded, oldCreate.await())
    assertNull(
      actions.create<co.typie.graphql.fragment.NoteCard_note>(newRequest) {
        error("a second create must remain blocked")
      }
    )

    finishNewCreate.complete(Unit)
    assertEquals(NoteActionOutcome.Success(newNote), newCreate.await())
  }

  @Test
  fun `cancelled deletion releases pending state without converging the note`() = runTest {
    val siteId = "cancelled-delete-site"
    val note = notesNote(id = "cancelled-delete-note", siteId = siteId)
    val converged = mutableListOf<String>()
    val mutationStarted = CompletableDeferred<Unit>()
    val actions = NoteActions()
    actions.activate(
      siteId = siteId,
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = { converged += it },
    )

    val deletion = launch {
      actions.delete(request = actions.captureRequest()!!, noteId = note.id) {
        mutationStarted.complete(Unit)
        awaitCancellation()
      }
    }
    mutationStarted.await()

    assertTrue(actions.isPendingDeletion(note.id))
    assertNull(
      actions.delete(request = actions.captureRequest()!!, noteId = note.id) {
        error("a duplicate deletion must not run")
      }
    )

    deletion.cancelAndJoin()

    assertFalse(actions.isPendingDeletion(note.id))
    assertEquals(emptyList(), converged)
  }

  @Test
  fun `completion from replaced deletion cannot clear the new owner pending state`() = runTest {
    val noteId = "reused-delete-note"
    val oldMutationStarted = CompletableDeferred<Unit>()
    val finishOldMutation = CompletableDeferred<Unit>()
    val newMutationStarted = CompletableDeferred<Unit>()
    val finishNewMutation = CompletableDeferred<Unit>()
    val actions = NoteActions()
    actions.activate(
      siteId = "old-delete-site",
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = {},
    )
    val oldRequest = actions.captureRequest()!!
    val oldDeletion = async {
      actions.delete(request = oldRequest, noteId = noteId) {
        oldMutationStarted.complete(Unit)
        finishOldMutation.await()
        Result.Exception(Exception("late failure"))
      }
    }
    oldMutationStarted.await()

    actions.activate(
      siteId = "new-delete-site",
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = {},
    )
    val newRequest = actions.captureRequest()!!
    val newDeletion = async {
      actions.delete(request = newRequest, noteId = noteId) {
        newMutationStarted.complete(Unit)
        finishNewMutation.await()
        Result.Exception(Exception("current failure"))
      }
    }
    newMutationStarted.await()
    finishOldMutation.complete(Unit)

    assertEquals(NoteActionOutcome.Superseded, oldDeletion.await())
    assertTrue(actions.isPendingDeletion(noteId))

    finishNewMutation.complete(Unit)
    assertIs<NoteActionOutcome.Failure>(newDeletion.await())
    assertFalse(actions.isPendingDeletion(noteId))
  }

  @Test
  fun `cancelled status change restores presence and releases pending state`() = runTest {
    val siteId = "cancelled-status-site"
    val note =
      notesNote(id = "cancelled-status-note", siteId = siteId, status = NoteStatus.RESOLVED)
    val sourceState = NoteListState(NoteStatus.RESOLVED)
    val destinationState = NoteListState(NoteStatus.OPEN)
    sourceState.sync(listOf(note))
    val mutationStarted = CompletableDeferred<Unit>()
    val actions = NoteActions()
    actions.activate(
      siteId = siteId,
      entityId = null,
      editState = createNoteEditState(),
      onTerminal = {},
    )

    val statusChange = launch {
      actions.toggleStatus(
        request = actions.captureRequest()!!,
        note = note,
        sourceState = sourceState,
        destinationState = destinationState,
        beforeMutation = { true },
      ) { _, _ ->
        mutationStarted.complete(Unit)
        awaitCancellation()
      }
    }
    mutationStarted.await()

    assertTrue(actions.isChangingStatus(note.id))
    assertTrue(sourceState.isExiting(note.id))
    assertNull(
      actions.toggleStatus(
        request = actions.captureRequest()!!,
        note = note,
        sourceState = sourceState,
        destinationState = destinationState,
        beforeMutation = { error("a duplicate status change must remain blocked") },
      ) { _, _ ->
        error("a duplicate status mutation must not run")
      }
    )

    statusChange.cancelAndJoin()

    assertFalse(actions.isChangingStatus(note.id))
    assertFalse(sourceState.isExiting(note.id))
    assertEquals(listOf(note.id), sourceState.merge(listOf(note)).map { it.id })
  }

  @Test
  fun `authoritative source reentry after a successful status mutation restores presence`() =
    runTest {
      val siteId = "successful-status-site"
      val note = notesNote(id = "successful-status-note", siteId = siteId)
      val openState = NoteListState(NoteStatus.OPEN)
      val resolvedState = NoteListState(NoteStatus.RESOLVED)
      openState.sync(listOf(note))
      val actions = NoteActions()
      actions.activate(
        siteId = siteId,
        entityId = null,
        editState = createNoteEditState(),
        onTerminal = {},
      )

      val outcome =
        actions.toggleStatus(
          request = actions.captureRequest()!!,
          note = note,
          sourceState = openState,
          destinationState = resolvedState,
          beforeMutation = { true },
        ) { _, status ->
          Result.Ok(note.copy(status = status))
        }

      assertIs<NoteActionOutcome.Success<*>>(outcome)
      assertFalse(actions.isChangingStatus(note.id))
      assertTrue(openState.isExiting(note.id))
      assertEquals(emptyList(), resolvedState.merge(emptyList()))

      openState.finishExiting(note.id)
      openState.sync(listOf(note))

      assertFalse(openState.isExiting(note.id))
      assertTrue(openState.isEntering(note.id))
      assertEquals(listOf(note.id), openState.merge(listOf(note)).map { it.id })
    }
}
