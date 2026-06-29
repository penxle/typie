package co.typie.graphql

import co.typie.contract.LoadableState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineExceptionHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withContext

@OptIn(ExperimentalCoroutinesApi::class)
class LoadableMutationTest {
  @Test
  fun `replaced in-flight completion cannot overwrite newer success`() =
    runTest(StandardTestDispatcher()) {
      val mutation = LoadableMutation<String>()
      val first = CompletableDeferred<String>()
      val second = CompletableDeferred<String>()
      val successes = mutableListOf<String>()

      mutation.run(
        scope = this,
        replaceInFlight = true,
        block = { withContext(NonCancellable) { first.await() } },
        onSuccess = successes::add,
      )
      advanceUntilIdle()

      mutation.run(
        scope = this,
        replaceInFlight = true,
        block = { second.await() },
        onSuccess = successes::add,
      )
      advanceUntilIdle()

      second.complete("second")
      advanceUntilIdle()
      first.complete("first")
      advanceUntilIdle()

      assertEquals("second", mutation.data)
      assertEquals(null, mutation.error)
      assertEquals(listOf("second"), successes)
    }

  @Test
  fun `replaced in-flight error cannot overwrite newer success`() =
    runTest(StandardTestDispatcher()) {
      val mutation = LoadableMutation<String>()
      val first = CompletableDeferred<String>()
      val second = CompletableDeferred<String>()
      val errors = mutableListOf<Throwable>()

      mutation.run(
        scope = this,
        replaceInFlight = true,
        block = { withContext(NonCancellable) { first.await() } },
        onError = errors::add,
      )
      advanceUntilIdle()

      mutation.run(scope = this, replaceInFlight = true, block = { second.await() })
      advanceUntilIdle()

      second.complete("second")
      advanceUntilIdle()
      first.completeExceptionally(IllegalStateException("first"))
      advanceUntilIdle()

      assertEquals("second", mutation.data)
      assertEquals(null, mutation.error)
      assertEquals(emptyList(), errors)
    }

  @Test
  fun `reset prevents stale completion from updating state or callbacks`() =
    runTest(StandardTestDispatcher()) {
      val mutation = LoadableMutation<String>()
      val pending = CompletableDeferred<String>()
      val successes = mutableListOf<String>()

      mutation.run(
        scope = this,
        block = { withContext(NonCancellable) { pending.await() } },
        onSuccess = successes::add,
      )
      advanceUntilIdle()

      mutation.reset()
      pending.complete("done")
      advanceUntilIdle()

      assertEquals(LoadableState.Idle, mutation.state)
      assertEquals(emptyList<String>(), successes)
    }

  @Test
  fun `success callback failure does not turn successful mutation into loader error`() =
    runTest(StandardTestDispatcher()) {
      val mutation = LoadableMutation<String>()
      val failure = IllegalStateException("callback")
      val failures = mutableListOf<Throwable>()
      val scope =
        CoroutineScope(
          coroutineContext +
            SupervisorJob() +
            CoroutineExceptionHandler { _, error -> failures += error }
        )

      try {
        mutation.run(scope = scope, block = { "done" }, onSuccess = { throw failure })
        advanceUntilIdle()
      } finally {
        scope.cancel()
      }

      assertEquals("done", mutation.data)
      assertEquals(null, mutation.error)
      assertEquals(listOf<Throwable>(failure), failures)
    }
}
