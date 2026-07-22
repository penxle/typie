package co.typie.ui.component.sheet

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class SheetTest {
  @Test
  fun acceptsInputUntilTheLastEntryBeginsDismissal() = runTest {
    val sheet = Sheet()

    assertFalse(sheet.acceptsInput)

    val first = async { sheet.present<String> {} }
    val second = async { sheet.present<String> {} }
    advanceUntilIdle()
    val firstEntry = sheet.entries[0]
    val secondEntry = sheet.entries[1]

    assertTrue(sheet.acceptsInput)

    sheet.stopEntryAcceptingInput(firstEntry)
    assertTrue(sheet.acceptsInput)

    sheet.stopEntryAcceptingInput(secondEntry)
    assertFalse(sheet.acceptsInput)

    sheet.resolveEntry(firstEntry, "first")
    advanceUntilIdle()
    assertEquals("first", first.await())
    assertFalse(sheet.acceptsInput)

    sheet.resolveEntry(secondEntry, "second")
    advanceUntilIdle()
    assertEquals("second", second.await())
    assertFalse(sheet.acceptsInput)
  }

  @Test
  fun stoppingEntryIsIdempotent() = runTest {
    val sheet = Sheet()
    val result = async { sheet.present<Unit> {} }
    advanceUntilIdle()
    val entry = sheet.entries.single()

    sheet.stopEntryAcceptingInput(entry)
    sheet.stopEntryAcceptingInput(entry)

    assertFalse(sheet.acceptsInput)
    sheet.resolveEntry(entry, Unit)
    advanceUntilIdle()
    assertEquals(Unit, result.await())
  }
}
