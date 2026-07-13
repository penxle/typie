package co.typie.editor

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.yield

class PriorityMutexTest {
  @Test
  fun priorityLockKeepsFifoOrderWithinEscalationWindow() = runTest {
    val mutex = PriorityMutex()
    val order = mutableListOf<String>()
    val release = CompletableDeferred<Unit>()

    launch {
      mutex.withLock {
        order += "holder"
        release.await()
      }
    }
    runCurrent()
    launch { mutex.withLock { order += "queued" } }
    runCurrent()
    launch { mutex.withPriorityLock { order += "priority" } }
    runCurrent()

    release.complete(Unit)
    advanceUntilIdle()

    assertEquals(listOf("holder", "queued", "priority"), order)
  }

  @Test
  fun priorityLockOvertakesQueuedLocksAfterEscalation() = runTest {
    val mutex = PriorityMutex()
    val order = mutableListOf<String>()
    val release = CompletableDeferred<Unit>()

    launch {
      mutex.withLock {
        order += "holder"
        release.await()
      }
    }
    runCurrent()
    launch { mutex.withLock { order += "queued1" } }
    launch { mutex.withLock { order += "queued2" } }
    runCurrent()
    launch { mutex.withPriorityLock { order += "priority" } }
    runCurrent()
    advanceTimeBy(200)
    runCurrent()

    release.complete(Unit)
    advanceUntilIdle()

    assertEquals(listOf("holder", "priority", "queued1", "queued2"), order)
  }

  @Test
  fun escalatedPriorityLockDoesNotOvertakeLockAlreadyPastGate() = runTest {
    val mutex = PriorityMutex()
    val order = mutableListOf<String>()
    val releaseFirst = CompletableDeferred<Unit>()
    val releaseHolder = CompletableDeferred<Unit>()

    launch {
      mutex.withLock {
        order += "first"
        releaseFirst.await()
      }
    }
    runCurrent()
    launch {
      mutex.withPriorityLock {
        order += "holder"
        releaseHolder.await()
      }
    }
    runCurrent()
    advanceTimeBy(200)
    runCurrent()
    releaseFirst.complete(Unit)
    runCurrent()

    launch { mutex.withLock { order += "queued" } }
    runCurrent()
    launch { mutex.withPriorityLock { order += "priority" } }
    runCurrent()
    advanceTimeBy(200)
    runCurrent()

    releaseHolder.complete(Unit)
    advanceUntilIdle()

    assertEquals(listOf("first", "holder", "queued", "priority"), order)
  }

  @Test
  fun locksAreMutuallyExclusive() = runTest {
    val mutex = PriorityMutex()
    var active = 0
    var maxActive = 0
    repeat(12) { i ->
      launch {
        val body: suspend () -> Unit = {
          active += 1
          maxActive = maxOf(maxActive, active)
          yield()
          active -= 1
        }
        if (i % 3 == 0) mutex.withPriorityLock(body) else mutex.withLock(body)
      }
    }
    advanceUntilIdle()

    assertEquals(1, maxActive)
    assertEquals(0, active)
  }
}
