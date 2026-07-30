package co.typie.domain.note

import co.typie.ext.EdgeAutoScrollController
import co.typie.graphql.type.NoteStatus
import co.typie.result.Result
import co.typie.ui.component.reorder.ReorderDrop
import co.typie.ui.component.reorder.ReorderableColumnState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class NoteReorderStateTest {
  @Test
  fun `the dragged note is committed in one move when it explains the desired order`() = runTest {
    val reorderState = reorderableState(listOf("a", "b", "c"))
    val state = NoteReorderState(scope = this, reorderState = reorderState)
    val moves = mutableListOf<MoveCall>()
    state.sync(
      identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN),
      notes =
        listOf(
          notesNote(id = "a", order = "100"),
          notesNote(id = "b", order = "200"),
          notesNote(id = "c", order = "300"),
        ),
    )

    state.commit(
      drop =
        ReorderDrop(movedKey = "a", fromIndex = 0, toIndex = 2, orderedKeys = listOf("b", "c", "a"))
    ) { note, lowerOrder, upperOrder ->
      moves += MoveCall(note.id, lowerOrder, upperOrder)
      Result.Ok("400")
    }
    runCurrent()

    assertEquals(listOf(MoveCall("a", lowerOrder = "300", upperOrder = null)), moves)
  }

  @Test
  fun `a second drop replaces desired order while the first request is in flight`() = runTest {
    val reorderState = reorderableState(listOf("a", "b", "c"))
    val state = NoteReorderState(scope = this, reorderState = reorderState)
    val firstMoveStarted = CompletableDeferred<Unit>()
    val finishFirstMove = CompletableDeferred<Unit>()
    val moves = mutableListOf<MoveCall>()
    state.sync(
      identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN),
      notes =
        listOf(
          notesNote(id = "a", order = "100"),
          notesNote(id = "b", order = "200"),
          notesNote(id = "c", order = "300"),
        ),
    )
    val moveNote:
      suspend (co.typie.graphql.fragment.NoteCard_note, String?, String?) -> Result<
          String,
          Nothing,
        > =
      { note, lowerOrder, upperOrder ->
        moves += MoveCall(note.id, lowerOrder, upperOrder)
        if (moves.size == 1) {
          firstMoveStarted.complete(Unit)
          finishFirstMove.await()
          Result.Ok("400")
        } else {
          Result.Ok("350")
        }
      }

    state.commit(
      ReorderDrop("a", fromIndex = 0, toIndex = 2, orderedKeys = listOf("b", "c", "a")),
      moveNote,
    )
    runCurrent()
    firstMoveStarted.await()

    reorderState.resetOrder(listOf("c", "b", "a"))
    state.commit(
      ReorderDrop("b", fromIndex = 0, toIndex = 1, orderedKeys = listOf("c", "b", "a")),
      moveNote,
    )
    finishFirstMove.complete(Unit)
    runCurrent()

    assertEquals(
      listOf(
        MoveCall("a", lowerOrder = "300", upperOrder = null),
        MoveCall("b", lowerOrder = "300", upperOrder = "400"),
      ),
      moves,
    )
  }

  @Test
  fun `failure restores the latest authoritative order observed during the request`() = runTest {
    val reorderState = reorderableState(listOf("a", "b", "c"))
    val state = NoteReorderState(scope = this, reorderState = reorderState)
    val moveStarted = CompletableDeferred<Unit>()
    val finishMove = CompletableDeferred<Unit>()
    val failures = mutableListOf<Throwable>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.failures.collect { failures += it }
    }
    state.sync(
      identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN),
      notes =
        listOf(
          notesNote(id = "a", order = "100"),
          notesNote(id = "b", order = "200"),
          notesNote(id = "c", order = "300"),
        ),
    )

    state.commit(
      ReorderDrop("a", fromIndex = 0, toIndex = 2, orderedKeys = listOf("b", "c", "a"))
    ) { _, _, _ ->
      moveStarted.complete(Unit)
      finishMove.await()
      Result.Exception(Exception("offline"))
    }
    runCurrent()
    moveStarted.await()

    state.sync(
      identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN),
      notes =
        listOf(
          notesNote(id = "c", order = "050"),
          notesNote(id = "a", order = "100"),
          notesNote(id = "b", order = "200"),
        ),
    )
    finishMove.complete(Unit)
    runCurrent()

    assertEquals(listOf("c", "a", "b"), reorderState.keys)
    assertEquals(1, failures.size)
  }

  @Test
  fun `late failure is ignored after authoritative order reaches the desired order`() = runTest {
    val reorderState = reorderableState(listOf("a", "b", "c"))
    val state = NoteReorderState(scope = this, reorderState = reorderState)
    val moveStarted = CompletableDeferred<Unit>()
    val finishMove = CompletableDeferred<Unit>()
    val failures = mutableListOf<Throwable>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.failures.collect { failures += it }
    }
    val identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN)
    state.sync(
      identity = identity,
      notes =
        listOf(
          notesNote(id = "a", order = "100"),
          notesNote(id = "b", order = "200"),
          notesNote(id = "c", order = "300"),
        ),
    )

    reorderState.resetOrder(listOf("b", "c", "a"))
    state.commit(
      ReorderDrop("a", fromIndex = 0, toIndex = 2, orderedKeys = listOf("b", "c", "a"))
    ) { _, _, _ ->
      moveStarted.complete(Unit)
      finishMove.await()
      Result.Exception(Exception("late failure"))
    }
    runCurrent()
    moveStarted.await()

    state.sync(
      identity = identity,
      notes =
        listOf(
          notesNote(id = "b", order = "100"),
          notesNote(id = "c", order = "200"),
          notesNote(id = "a", order = "300"),
        ),
    )
    finishMove.complete(Unit)
    runCurrent()

    assertEquals(listOf("b", "c", "a"), reorderState.keys)
    assertTrue(failures.isEmpty())
  }

  @Test
  fun `terminal deletion stops an in flight reorder without reporting failure`() = runTest {
    val siteId = "terminal-reorder-site"
    val reorderState = reorderableState(listOf("a", "b"))
    val state = NoteReorderState(scope = this, reorderState = reorderState)
    val moveStarted = CompletableDeferred<Unit>()
    val finishMove = CompletableDeferred<Unit>()
    val failures = mutableListOf<Throwable>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.failures.collect { failures += it }
    }
    state.sync(
      identity = NoteListIdentity(siteId = siteId, status = NoteStatus.OPEN),
      notes = listOf(notesNote(id = "a", order = "100"), notesNote(id = "b", order = "200")),
    )

    state.commit(ReorderDrop("a", fromIndex = 0, toIndex = 1, orderedKeys = listOf("b", "a"))) {
      _,
      _,
      _ ->
      moveStarted.complete(Unit)
      finishMove.await()
      Result.Exception(Exception("offline"))
    }
    runCurrent()
    moveStarted.await()

    NoteSync.publish(NoteUpdate(co.typie.graphql.type.NoteUpdateKind.DELETED, "a", siteId))
    finishMove.complete(Unit)
    runCurrent()

    assertTrue(failures.isEmpty())
    NoteSync.publish(NoteUpdate(co.typie.graphql.type.NoteUpdateKind.CREATED, "a", siteId))
  }

  @Test
  fun `membership change supersedes an in flight reorder and its late failure`() = runTest {
    val reorderState = reorderableState(listOf("a", "b"))
    val state = NoteReorderState(scope = this, reorderState = reorderState)
    val moveStarted = CompletableDeferred<Unit>()
    val finishMove = CompletableDeferred<Unit>()
    val failures = mutableListOf<Throwable>()
    backgroundScope.launch(UnconfinedTestDispatcher(testScheduler)) {
      state.failures.collect { failures += it }
    }
    val identity = NoteListIdentity(siteId = "site", status = NoteStatus.OPEN)
    state.sync(
      identity = identity,
      notes = listOf(notesNote(id = "a", order = "100"), notesNote(id = "b", order = "200")),
    )

    state.commit(ReorderDrop("a", fromIndex = 0, toIndex = 1, orderedKeys = listOf("b", "a"))) {
      _,
      _,
      _ ->
      moveStarted.complete(Unit)
      finishMove.await()
      Result.Exception(Exception("late failure"))
    }
    runCurrent()
    moveStarted.await()

    state.sync(
      identity = identity,
      notes =
        listOf(
          notesNote(id = "a", order = "100"),
          notesNote(id = "b", order = "200"),
          notesNote(id = "c", order = "300"),
        ),
    )
    finishMove.complete(Unit)
    runCurrent()

    assertEquals(listOf("a", "b", "c"), reorderState.keys)
    assertTrue(failures.isEmpty())
  }
}

private data class MoveCall(val noteId: String, val lowerOrder: String?, val upperOrder: String?)

private fun reorderableState(keys: List<String>): ReorderableColumnState<String> =
  ReorderableColumnState<String>(
      edgeAutoScrollController =
        EdgeAutoScrollController(verticalScrollableState = null, horizontalScrollableState = null)
    )
    .also { it.inputKeys = keys }
