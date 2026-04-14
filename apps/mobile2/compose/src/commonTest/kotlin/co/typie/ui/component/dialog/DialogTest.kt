package co.typie.ui.component.dialog

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class DialogTest {

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
