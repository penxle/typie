package co.typie.ui.component.bottomsheet

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest

class BottomSheetHostStateTest {

  @Test
  fun initiallyEmpty() {
    val state = BottomSheetHostState()
    assertTrue(state.entries.isEmpty())
  }

  @Test
  fun showAddsEntry() = runTest {
    val state = BottomSheetHostState()
    val job = launch {
      state.show<Unit> { /* composable content */ }
    }
    testScheduler.advanceUntilIdle()
    assertEquals(1, state.entries.size)
    job.cancel()
  }

  @Test
  fun showReturnsResult() = runTest {
    val state = BottomSheetHostState()
    val job = launch {
      val result = state.show<String> { /* composable content */ }
      assertEquals("hello", result)
    }
    testScheduler.advanceUntilIdle()
    @Suppress("UNCHECKED_CAST")
    (state.entries.first() as BottomSheetEntry<String>).resume("hello")
    testScheduler.advanceUntilIdle()
    job.join()
  }

  @Test
  fun dismissRemovesEntry() = runTest {
    val state = BottomSheetHostState()
    val job = launch {
      state.show<Unit> { /* composable content */ }
    }
    testScheduler.advanceUntilIdle()
    @Suppress("UNCHECKED_CAST")
    (state.entries.first() as BottomSheetEntry<Unit>).resume(Unit)
    testScheduler.advanceUntilIdle()
    assertTrue(state.entries.isEmpty())
    job.join()
  }

  @Test
  fun multipleEntriesStack() = runTest {
    val state = BottomSheetHostState()
    val job1 = launch { state.show<Unit> { /* no dismiss */ } }
    val job2 = launch { state.show<Unit> { /* no dismiss */ } }
    testScheduler.advanceUntilIdle()
    assertEquals(2, state.entries.size)
    job1.cancel()
    job2.cancel()
  }
}
