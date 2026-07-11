package co.typie.editor.input

import androidx.compose.ui.input.key.Key as ComposeKey
import co.typie.editor.KeyBinding
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import co.typie.platform.ClipboardReadPayload
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.runTest

class EditorKeyBindingCoalescerTest {
  private class FakeClipboard : Clipboard {
    override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = false

    override suspend fun copy(text: String, mimeType: String): Boolean = false

    override suspend fun copyRichText(html: String, text: String): Boolean = false

    override suspend fun paste(): ClipboardReadPayload? = null
  }

  private data class Dispatched(
    val messages: List<Message>,
    val target: EditorBringIntoViewTarget?,
  )

  private class Harness(scope: CoroutineScope) {
    val dispatched = mutableListOf<Dispatched>()
    var dispatchGate: CompletableDeferred<Unit>? = null
    private val clipboard = FakeClipboard()
    private val messagesByBinding = mutableMapOf<KeyBinding, List<Message>>()
    private val coalescer =
      EditorKeyBindingCoalescer(
        scope = scope,
        resolveMessages = { binding, _ -> messagesByBinding.getValue(binding) },
        dispatch = { messages, target ->
          dispatched += Dispatched(messages, target)
          dispatchGate?.let { gate ->
            dispatchGate = null
            gate.await()
          }
        },
      )

    fun submit(
      message: Message,
      coalescible: Boolean = true,
      target: EditorBringIntoViewTarget? = EditorBringIntoViewTarget.CurrentSelectionHead,
    ) {
      val binding =
        KeyBinding(
          key = ComposeKey.DirectionRight,
          bringIntoViewTarget = target,
          coalescible = coalescible,
          action = { emptyList() },
        )
      messagesByBinding[binding] = listOf(message)
      coalescer.submit(binding, clipboard)
    }
  }

  private fun moveMessage(index: Int): Message =
    Message.Navigation(
      NavigationOp.Move(Movement.Grapheme(Direction.Forward), extend = index % 2 == 0)
    )

  private fun runCoalescerTest(block: suspend TestScope.(Harness) -> Unit) = runTest {
    val workerScope = CoroutineScope(coroutineContext + Job())
    try {
      block(Harness(workerScope))
    } finally {
      workerScope.cancel()
    }
  }

  @Test
  fun `queued coalescible bindings drain into a single dispatch`() = runCoalescerTest { harness ->
    val messages = List(5) { moveMessage(it) }

    messages.forEach { harness.submit(it) }
    testScheduler.advanceUntilIdle()

    assertEquals(
      listOf(Dispatched(messages, EditorBringIntoViewTarget.CurrentSelectionHead)),
      harness.dispatched,
    )
  }

  @Test
  fun `non-coalescible binding splits the batch preserving order`() = runCoalescerTest { harness ->
    val first = moveMessage(1)
    val exclusive = moveMessage(2)
    val second = moveMessage(3)

    harness.submit(first)
    harness.submit(exclusive, coalescible = false, target = null)
    harness.submit(second)
    testScheduler.advanceUntilIdle()

    assertEquals(
      listOf(
        Dispatched(listOf(first), EditorBringIntoViewTarget.CurrentSelectionHead),
        Dispatched(listOf(exclusive), null),
        Dispatched(listOf(second), EditorBringIntoViewTarget.CurrentSelectionHead),
      ),
      harness.dispatched,
    )
  }

  @Test
  fun `batch keeps last non-null bring-into-view target`() = runCoalescerTest { harness ->
    val first = moveMessage(1)
    val second = moveMessage(2)

    harness.submit(first)
    harness.submit(second, target = null)
    testScheduler.advanceUntilIdle()

    assertEquals(
      listOf(Dispatched(listOf(first, second), EditorBringIntoViewTarget.CurrentSelectionHead)),
      harness.dispatched,
    )
  }

  @Test
  fun `bindings submitted during an active dispatch drain in the next batch`() =
    runCoalescerTest { harness ->
      val gate = CompletableDeferred<Unit>()
      harness.dispatchGate = gate
      val first = moveMessage(1)

      harness.submit(first)
      testScheduler.runCurrent()

      val late = List(3) { moveMessage(it + 2) }
      late.forEach { harness.submit(it) }
      testScheduler.runCurrent()
      gate.complete(Unit)
      testScheduler.advanceUntilIdle()

      assertEquals(
        listOf(
          Dispatched(listOf(first), EditorBringIntoViewTarget.CurrentSelectionHead),
          Dispatched(late, EditorBringIntoViewTarget.CurrentSelectionHead),
        ),
        harness.dispatched,
      )
    }
}
