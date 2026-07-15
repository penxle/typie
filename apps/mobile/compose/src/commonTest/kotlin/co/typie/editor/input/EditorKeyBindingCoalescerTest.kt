package co.typie.editor.input

import androidx.compose.ui.input.key.Key as ComposeKey
import co.typie.editor.EditorLocalEditCoordinator
import co.typie.editor.KeyBinding
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import co.typie.platform.ClipboardReadPayload
import kotlin.coroutines.CoroutineContext
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
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

  private data class Dispatched(val messages: List<Message>, val target: EditorBringIntoViewTarget?)

  private class Harness(
    private val scope: CoroutineScope,
    private val onDispatch: suspend () -> Unit,
  ) {
    val dispatched = mutableListOf<Dispatched>()
    var dispatchGate: CompletableDeferred<Unit>? = null
    var dispatchFailure: Throwable? = null
    private val clipboard = FakeClipboard()
    private val messagesByBinding = mutableMapOf<KeyBinding, List<Message>>()
    private val coalescer =
      EditorKeyBindingCoalescer(
        scope = scope,
        resolveMessages = { binding, _ -> messagesByBinding.getValue(binding) },
        dispatch = { messages, target ->
          onDispatch()
          dispatched += Dispatched(messages, target)
          dispatchGate?.let { gate ->
            dispatchGate = null
            gate.await()
          }
          dispatchFailure?.let { failure ->
            dispatchFailure = null
            throw failure
          }
        },
      )

    fun submit(
      message: Message,
      coalescible: Boolean = true,
      target: EditorBringIntoViewTarget? = EditorBringIntoViewTarget.CurrentSelectionHead,
      localEditContext: CoroutineContext? = null,
    ): Deferred<Unit> = submit(listOf(message), coalescible, target, localEditContext)

    fun submit(
      messages: List<Message>,
      coalescible: Boolean = true,
      target: EditorBringIntoViewTarget? = EditorBringIntoViewTarget.CurrentSelectionHead,
      localEditContext: CoroutineContext? = null,
    ): Deferred<Unit> {
      val binding =
        KeyBinding(
          key = ComposeKey.DirectionRight,
          bringIntoViewTarget = target,
          coalescible = coalescible,
          action = { emptyList() },
        )
      messagesByBinding[binding] = messages
      return coalescer.submit(binding, clipboard, localEditContext)
    }

    fun cancel() {
      scope.cancel()
    }
  }

  private fun moveMessage(index: Int): Message =
    Message.Navigation(
      NavigationOp.Move(Movement.Grapheme(Direction.Forward), extend = index % 2 == 0)
    )

  private fun runCoalescerTest(
    onDispatch: suspend () -> Unit = {},
    block: suspend TestScope.(Harness) -> Unit,
  ) = runTest {
    val workerScope = CoroutineScope(coroutineContext + Job())
    try {
      block(Harness(workerScope, onDispatch))
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
  fun `empty coalescible binding completes without dispatch`() = runCoalescerTest { harness ->
    val completion = harness.submit(messages = emptyList())

    testScheduler.runCurrent()

    assertTrue(completion.isCompleted)
    assertEquals(emptyList(), harness.dispatched)
  }

  @Test
  fun `accepted local edit reaches dispatch across the worker coroutine`() {
    val coordinator = EditorLocalEditCoordinator()
    val localEdit = assertNotNull(coordinator.register())
    val quiescence = coordinator.quiesce()
    var dispatchedWithinLocalEdit = false

    runCoalescerTest(onDispatch = { coordinator.run { dispatchedWithinLocalEdit = true } }) {
      harness ->
      harness.submit(moveMessage(1), localEditContext = localEdit).await()

      assertTrue(dispatchedWithinLocalEdit)
      localEdit.complete()
      assertTrue(quiescence.await().isSuccess)
    }
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

  @Test
  fun `dispatch failure does not strand bindings queued for the next batch`() =
    runCoalescerTest { harness ->
      val gate = CompletableDeferred<Unit>()
      harness.dispatchGate = gate
      harness.dispatchFailure = IllegalStateException("dispatch failed")

      val failed = harness.submit(moveMessage(1))
      testScheduler.runCurrent()
      val queued = harness.submit(moveMessage(2))
      testScheduler.runCurrent()

      gate.complete(Unit)
      testScheduler.advanceUntilIdle()

      assertFailsWith<IllegalStateException> { failed.await() }
      assertFalse(queued.isCancelled)
      queued.await()
    }

  @Test
  fun `dispatch cancellation does not stop the worker with another binding queued`() =
    runCoalescerTest { harness ->
      val gate = CompletableDeferred<Unit>()
      harness.dispatchGate = gate
      harness.dispatchFailure = CancellationException("dispatch cancelled")

      val cancelled = harness.submit(moveMessage(1))
      testScheduler.runCurrent()
      val queued = harness.submit(moveMessage(2))
      testScheduler.runCurrent()

      gate.complete(Unit)
      testScheduler.advanceUntilIdle()

      assertTrue(cancelled.isCancelled)
      assertTrue(queued.isCompleted)
    }

  @Test
  fun `worker cancellation terminates active and queued submissions`() =
    runCoalescerTest { harness ->
      harness.dispatchGate = CompletableDeferred()
      val active = harness.submit(moveMessage(1))
      testScheduler.runCurrent()
      val queued = harness.submit(moveMessage(2))

      harness.cancel()
      testScheduler.runCurrent()

      assertTrue(active.isCancelled)
      assertTrue(queued.isCancelled)
    }

  @Test
  fun `submit after worker stop fails the completion instead of throwing`() =
    runCoalescerTest { harness ->
      testScheduler.runCurrent()
      harness.cancel()
      testScheduler.advanceUntilIdle()

      val late = harness.submit(moveMessage(1))

      assertTrue(late.isCancelled)
      assertEquals(emptyList(), harness.dispatched)
    }
}
