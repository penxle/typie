package co.typie.screen.subscription

// cspell:ignore UNDISPATCHED

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalCoroutinesApi::class)
class SubscriptionSyncTest {
  @Test
  fun `subscription sync delivers events to multiple subscribers in order`() = runTest {
    val sync = SubscriptionSync()
    val first = backgroundScope.async(start = CoroutineStart.UNDISPATCHED) {
      sync.events.take(2).toList()
    }
    val second = backgroundScope.async(start = CoroutineStart.UNDISPATCHED) {
      sync.events.take(2).toList()
    }

    advanceUntilIdle()

    sync.notifyChanged()
    sync.notifyChanged()
    advanceUntilIdle()

    assertEquals(listOf(1, 2), first.await())
    assertEquals(listOf(1, 2), second.await())
  }
}
