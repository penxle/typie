package co.typie.ui.state

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

class AsyncActionTest {
  @Test
  fun `running reflects action lifecycle`() = runTest {
    val action = AsyncAction(this)
    val gate = CompletableDeferred<Unit>()

    action.launch { gate.await() }

    runCurrent()

    assertTrue(action.running)

    gate.complete(Unit)
    advanceUntilIdle()

    assertFalse(action.running)
  }

  @Test
  fun `launch ignores duplicate requests while running`() = runTest {
    val action = AsyncAction(this)
    val gate = CompletableDeferred<Unit>()
    var runs = 0

    action.launch {
      runs += 1
      gate.await()
    }
    runCurrent()

    action.launch { runs += 1 }
    runCurrent()

    assertEquals(1, runs)

    gate.complete(Unit)
    advanceUntilIdle()

    assertFalse(action.running)
    assertEquals(1, runs)
  }

  @Test
  fun `launch reports regular exceptions through onFailure`() = runTest {
    val action = AsyncAction(this)
    var failure: Exception? = null

    action.launch(onFailure = { failure = it }) { error("failed") }

    advanceUntilIdle()

    assertIs<IllegalStateException>(failure)
    assertFalse(action.running)
  }

  @Test
  fun `launch rethrows cancellation instead of reporting failure`() = runTest {
    val action = AsyncAction(this)
    var failure: Exception? = null

    action.launch(onFailure = { failure = it }) { throw CancellationException("cancelled") }

    advanceUntilIdle()

    assertNull(failure)
    assertFalse(action.running)
  }
}
