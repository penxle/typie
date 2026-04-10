package co.typie.ui.component.sheet

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest

class SheetOverlayPresenterStateTest {

  @Test
  fun presentAddsEntriesAndTracksTopOfStack() = runTest {
    val presenter = SheetOverlayPresenterState()

    val jobA = launch { presenter.present<Unit>(spec = SheetOverlaySpec()) {} }
    val jobB = launch {
      presenter.present<Unit>(spec = SheetOverlaySpec(mode = SheetMode.NonModalOverlay)) {}
    }

    testScheduler.advanceUntilIdle()

    assertEquals(2, presenter.entries.size)
    assertFalse(presenter.entries.first().isTopOfStack)
    assertTrue(presenter.entries.last().isTopOfStack)

    jobA.cancel()
    jobB.cancel()
  }

  @Test
  fun dismissReturnsDismissedResultAndRemovesEntry() = runTest {
    val presenter = SheetOverlayPresenterState()

    val job = launch {
      val result = presenter.present<Unit>(spec = SheetOverlaySpec()) {}
      assertEquals(SheetResult.Dismissed(SheetDismissReason.OutsideTap), result)
    }

    testScheduler.advanceUntilIdle()

    @Suppress("UNCHECKED_CAST") val entry = presenter.entries.single() as SheetOverlayEntry<Unit>
    entry.controller.dismiss(SheetDismissReason.OutsideTap)
    entry.resolve(requireNotNull(entry.controller.resolutionRequest))

    testScheduler.advanceUntilIdle()
    job.join()
    assertTrue(presenter.entries.isEmpty())
  }

  @Test
  fun completeReturnsValueAndClearsEntry() = runTest {
    val presenter = SheetOverlayPresenterState()

    val job = launch {
      val result = presenter.present<String>(spec = SheetOverlaySpec()) {}
      assertEquals(SheetResult.Completed("done"), result)
    }

    testScheduler.advanceUntilIdle()

    @Suppress("UNCHECKED_CAST")
    (presenter.entries.single() as SheetOverlayEntry<String>).let { entry ->
      entry.controller.complete("done")
      entry.resolve(requireNotNull(entry.controller.resolutionRequest))
    }

    testScheduler.advanceUntilIdle()
    job.join()
    assertTrue(presenter.entries.isEmpty())
  }

  @Test
  fun programmaticDismissHonorsDismissPolicy() = runTest {
    val presenter = SheetOverlayPresenterState()

    val job = launch {
      presenter.present<Unit>(
        spec = SheetOverlaySpec(dismissPolicy = SheetDismissPolicy(programmatic = false))
      ) {}
    }

    testScheduler.advanceUntilIdle()

    presenter.entries.single().controller.dismiss(SheetDismissReason.Programmatic)

    testScheduler.advanceUntilIdle()
    assertEquals(1, presenter.entries.size)

    job.cancel()
  }
}
