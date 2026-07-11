package co.typie.editor.input

import co.typie.editor.KeyBinding
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch

// Key repeats can outpace per-event editor dispatch; draining every queued
// coalescible binding into one dispatch keeps the pending work bounded to a
// single batch instead of one editor transaction per key event. Bindings whose
// actions read committed editor or clipboard state must not coalesce, because a
// batched action would otherwise run before the preceding messages commit.
internal class EditorKeyBindingCoalescer(
  private val scope: CoroutineScope,
  private val resolveMessages: suspend (KeyBinding, Clipboard) -> List<Message>,
  private val dispatch: suspend (List<Message>, EditorBringIntoViewTarget?) -> Unit,
) {
  private data class Submission(val binding: KeyBinding, val clipboard: Clipboard)

  private val submissions = Channel<Submission>(Channel.UNLIMITED)
  private var worker: Job? = null

  fun submit(binding: KeyBinding, clipboard: Clipboard) {
    ensureWorker()
    submissions.trySend(Submission(binding, clipboard))
  }

  private fun ensureWorker() {
    if (worker?.isActive == true) return
    worker = scope.launch {
      for (first in submissions) {
        drainFrom(first)
      }
    }
  }

  private suspend fun drainFrom(first: Submission) {
    val batch = mutableListOf<Message>()
    var batchTarget: EditorBringIntoViewTarget? = null

    suspend fun flushBatch() {
      if (batch.isEmpty()) return
      dispatch(batch.toList(), batchTarget)
      batch.clear()
      batchTarget = null
    }

    var current: Submission? = first
    while (current != null) {
      val binding = current.binding
      if (binding.coalescible) {
        batch += resolveMessages(binding, current.clipboard)
        binding.bringIntoViewTarget?.let { batchTarget = it }
      } else {
        flushBatch()
        dispatch(resolveMessages(binding, current.clipboard), binding.bringIntoViewTarget)
      }
      current = submissions.tryReceive().getOrNull()
    }
    flushBatch()
  }
}
