package co.typie.ui.component.sheet

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.runTest

class SheetTest {

  @Test
  fun presentReturnsCompletedValue() = runTest {
    val sheet = Sheet()
    var result: String? = "initial"

    val job = launch { result = sheet.present<String> {} }
    testScheduler.advanceUntilIdle()

    sheet.resolveEntry(sheet.entries.first(), "done")
    testScheduler.advanceUntilIdle()
    job.join()

    assertEquals("done", result)
  }

  @Test
  fun presentReturnsNullOnDismiss() = runTest {
    val sheet = Sheet()
    var result: String? = "initial"

    val job = launch { result = sheet.present<String> {} }
    testScheduler.advanceUntilIdle()

    sheet.resolveEntry(sheet.entries.first(), null)
    testScheduler.advanceUntilIdle()
    job.join()

    assertNull(result)
  }

  @Test
  fun entryAddedOnPresentAndRemovedOnResolve() = runTest {
    val sheet = Sheet()
    assertEquals(0, sheet.entries.size)

    val job = launch { sheet.present<Unit> {} }
    testScheduler.advanceUntilIdle()

    assertEquals(1, sheet.entries.size)

    sheet.resolveEntry(sheet.entries.first(), null)
    testScheduler.advanceUntilIdle()

    assertEquals(0, sheet.entries.size)
    job.join()
  }

  @Test
  fun multipleEntriesStack() = runTest {
    val sheet = Sheet()

    val job1 = launch { sheet.present<Unit> {} }
    testScheduler.advanceUntilIdle()
    val job2 = launch { sheet.present<Unit> {} }
    testScheduler.advanceUntilIdle()

    assertEquals(2, sheet.entries.size)

    sheet.resolveEntry(sheet.entries.last(), null)
    testScheduler.advanceUntilIdle()
    assertEquals(1, sheet.entries.size)

    sheet.resolveEntry(sheet.entries.first(), null)
    testScheduler.advanceUntilIdle()
    assertEquals(0, sheet.entries.size)

    job1.join()
    job2.join()
  }

  @Test
  fun stopsPassedToEntry() = runTest {
    val sheet = Sheet()
    val stops = listOf(SheetStop.Bottom(360.dp), SheetStop.Top(128.dp))

    launch { sheet.present<Unit>(stops = stops) {} }
    testScheduler.advanceUntilIdle()

    assertEquals(stops, sheet.entries.first().stops)
  }
}
