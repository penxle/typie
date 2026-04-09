package co.typie.service

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class SiteRefreshCoordinatorTest {

  @Test
  fun `coalescedSiteRefreshes filters out other sites`() = runTest {
    val signals = MutableSharedFlow<String>(extraBufferCapacity = 8)
    val refreshes = mutableListOf<Unit>()
    val job = launch {
      signals.coalescedSiteRefreshes(siteId = "site-a", debounceMs = 100).collect {
        refreshes += Unit
      }
    }
    runCurrent()

    signals.emit("site-b")
    advanceTimeBy(150)
    runCurrent()

    assertTrue(refreshes.isEmpty())

    job.cancel()
  }

  @Test
  fun `coalescedSiteRefreshes collapses a burst into one refetch`() = runTest {
    val signals = MutableSharedFlow<String>(extraBufferCapacity = 8)
    val refreshCount = mutableListOf<Unit>()
    val job = launch {
      signals.coalescedSiteRefreshes(siteId = "site-a", debounceMs = 100).collect {
        refreshCount += Unit
      }
    }
    runCurrent()

    signals.emit("site-a")
    advanceTimeBy(40)
    signals.emit("site-a")
    advanceTimeBy(40)
    signals.emit("site-a")
    advanceTimeBy(99)

    assertTrue(refreshCount.isEmpty())

    advanceTimeBy(1)
    runCurrent()

    assertEquals(1, refreshCount.size)

    job.cancel()
  }

  @Test
  fun `coalescedSiteRefreshes emits again for separated bursts`() = runTest {
    val signals = MutableSharedFlow<String>(extraBufferCapacity = 8)
    val refreshCount = mutableListOf<Unit>()
    val job = launch {
      signals.coalescedSiteRefreshes(siteId = "site-a", debounceMs = 100).collect {
        refreshCount += Unit
      }
    }
    runCurrent()

    signals.emit("site-a")
    advanceTimeBy(100)
    runCurrent()
    signals.emit("site-a")
    advanceTimeBy(100)
    runCurrent()

    assertEquals(2, refreshCount.size)

    job.cancel()
  }
}
