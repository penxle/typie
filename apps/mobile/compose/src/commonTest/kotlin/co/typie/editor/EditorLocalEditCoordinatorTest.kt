package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.joinAll
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(kotlinx.coroutines.ExperimentalCoroutinesApi::class)
class EditorLocalEditCoordinatorTest {
  @Test
  fun concurrentEditsAllReachTheQuiescenceBarrier() = runTest {
    val coordinator = EditorLocalEditCoordinator()
    val edits =
      List(8) {
          async(Dispatchers.Default) { List(1_000) { assertNotNull(coordinator.register()) } }
        }
        .awaitAll()
        .flatten()
    val quiescence = coordinator.quiesce()
    val result = async { quiescence.await() }

    edits
      .chunked(1_000)
      .map { chunk -> launch(Dispatchers.Default) { chunk.forEach(LocalEdit::complete) } }
      .joinAll()

    assertTrue(result.await().isSuccess)
  }

  @Test
  fun barrierWaitsForEveryAcceptedEdit() = runTest {
    val coordinator = EditorLocalEditCoordinator()
    val first = assertNotNull(coordinator.register())
    val second = assertNotNull(coordinator.register())
    val quiescence = coordinator.quiesce()
    val result = async { quiescence.await() }

    second.complete()
    runCurrent()
    assertFalse(result.isCompleted)

    first.complete()
    assertTrue(result.await().isSuccess)
  }

  @Test
  fun quiescenceRejectsNewEditsUntilResume() {
    val coordinator = EditorLocalEditCoordinator()
    val accepted = assertNotNull(coordinator.register())
    val quiescence = coordinator.quiesce()

    assertNull(coordinator.register())

    accepted.complete()
    quiescence.resume()
    assertNotNull(coordinator.register())
  }

  @Test
  fun barrierReportsAcceptedEditFailure() = runTest {
    val coordinator = EditorLocalEditCoordinator()
    val edit = assertNotNull(coordinator.register())
    val quiescence = coordinator.quiesce()
    val failure = IllegalStateException("commit failed")

    edit.fail(failure)

    val result = quiescence.await()
    assertTrue(result.isFailure)
    assertSame(failure, result.exceptionOrNull())
  }

  @Test
  fun barrierRemembersFailureButWaitsForRemainingAcceptedEdit() = runTest {
    val coordinator = EditorLocalEditCoordinator()
    val failed = assertNotNull(coordinator.register())
    val pending = assertNotNull(coordinator.register())
    val quiescence = coordinator.quiesce()
    val failure = IllegalStateException("commit failed")
    val result = async { quiescence.await() }

    failed.fail(failure)
    runCurrent()
    assertFalse(result.isCompleted)

    pending.complete()
    val settled = result.await()
    assertTrue(settled.isFailure)
    assertSame(failure, settled.exceptionOrNull())
  }
}
