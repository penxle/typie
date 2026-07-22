package co.typie.ui.component.dialog

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class DialogTest {

  @Test
  fun acceptsInputUntilTheLastQueuedEntryBeginsDismissal() = runTest {
    val dialog = Dialog()
    val first = async { dialog.present<Unit> {} }
    val second = async { dialog.present<Unit> {} }

    advanceUntilIdle()
    assertTrue(dialog.acceptsInput)

    dialog.stopEntryAcceptingInput(dialog.queue[0])
    assertTrue(dialog.acceptsInput)

    dialog.resolveCurrentEntry(DialogResult.Dismissed)
    advanceUntilIdle()
    assertTrue(dialog.acceptsInput)

    dialog.stopEntryAcceptingInput(dialog.queue[0])
    assertFalse(dialog.acceptsInput)

    dialog.resolveCurrentEntry(DialogResult.Dismissed)
    advanceUntilIdle()
    assertIs<DialogResult.Dismissed>(first.await())
    assertIs<DialogResult.Dismissed>(second.await())
  }

  @Test
  fun stoppingEntryIsIdempotent() = runTest {
    val dialog = Dialog()

    assertFalse(dialog.acceptsInput)

    val result = async { dialog.present<Unit> {} }
    advanceUntilIdle()
    val entry = dialog.queue.single()

    dialog.stopEntryAcceptingInput(entry)
    dialog.stopEntryAcceptingInput(entry)
    assertFalse(dialog.acceptsInput)

    dialog.resolveCurrentEntry(DialogResult.Dismissed)
    advanceUntilIdle()
    assertIs<DialogResult.Dismissed>(result.await())
  }

  @Test
  fun presentAndDismiss() = runTest {
    val dialog = Dialog()
    val result = async { dialog.present<Unit> {} }

    advanceUntilIdle()
    assertEquals(1, dialog.queue.size)
    assertEquals(dialog.queue.first(), dialog.current)

    dialog.resolveCurrentEntry(DialogResult.Dismissed)
    advanceUntilIdle()

    assertIs<DialogResult.Dismissed>(result.await())
    assertNull(dialog.current)
  }

  @Test
  fun presentAndResolve() = runTest {
    val dialog = Dialog()
    val result = async { dialog.present<String> {} }

    advanceUntilIdle()
    assertEquals(1, dialog.queue.size)

    dialog.resolveCurrentEntry(DialogResult.Resolved("done"))
    advanceUntilIdle()

    assertEquals(DialogResult.Resolved("done"), result.await())
    assertNull(dialog.current)
  }
}
