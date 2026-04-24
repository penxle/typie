package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class PendingSettleTest {
  private val dispatcher = StandardTestDispatcher()

  @Test
  fun await_completes_when_all_pages_committed_at_or_above_required_version() =
    runTest(dispatcher) {
      val pending = PendingSettle(setOf(0, 1), requiredVersion = 5L)
      var resumed = false
      val job =
        launch(dispatcher) {
          pending.await()
          resumed = true
        }
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(resumed)

      pending.markCommitted(0, 5L)
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(resumed)

      pending.markCommitted(1, 6L)
      dispatcher.scheduler.advanceUntilIdle()
      assertTrue(resumed)
      job.join()
    }

  @Test
  fun stale_version_commit_does_not_progress() =
    runTest(dispatcher) {
      val pending = PendingSettle(setOf(0), requiredVersion = 5L)
      var resumed = false
      val job =
        launch(dispatcher) {
          pending.await()
          resumed = true
        }
      pending.markCommitted(0, 4L) // stale
      dispatcher.scheduler.advanceUntilIdle()
      assertFalse(resumed)

      pending.markCommitted(0, 5L)
      dispatcher.scheduler.advanceUntilIdle()
      assertTrue(resumed)
      job.join()
    }

  @Test
  fun mark_detached_completes_without_render() =
    runTest(dispatcher) {
      val pending = PendingSettle(setOf(0, 1), requiredVersion = 3L)
      var resumed = false
      val job =
        launch(dispatcher) {
          pending.await()
          resumed = true
        }
      pending.markCommitted(0, 3L)
      pending.markDetached(1)
      dispatcher.scheduler.advanceUntilIdle()
      assertTrue(resumed)
      job.join()
    }

  @Test
  fun cancel_throws_cancellation() =
    runTest(dispatcher) {
      val pending = PendingSettle(setOf(0), requiredVersion = 1L)
      val job = launch(dispatcher) { kotlin.runCatching { pending.await() } }
      pending.cancel()
      dispatcher.scheduler.advanceUntilIdle()
      assertTrue(job.isCompleted)
    }
}
