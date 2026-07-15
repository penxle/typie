package co.typie.editor.input

import co.typie.editor.KeyBinding
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

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
  private data class Submission(
    val binding: KeyBinding,
    val clipboard: Clipboard,
    val localEditContext: CoroutineContext?,
    val completion: CompletableDeferred<Unit>,
  ) {
    fun complete() {
      completion.complete(Unit)
    }

    fun fail(error: Throwable) {
      completion.completeExceptionally(error)
    }
  }

  private inner class PendingBatch {
    private val submissions = mutableListOf<Submission>()
    private val messages = mutableListOf<Message>()
    private var target: EditorBringIntoViewTarget? = null

    fun add(submission: Submission, resolvedMessages: List<Message>) {
      submissions += submission
      messages += resolvedMessages
      submission.binding.bringIntoViewTarget?.let { target = it }
    }

    suspend fun flush() {
      if (submissions.isEmpty()) return
      if (messages.isNotEmpty()) {
        val context = submissions.firstNotNullOfOrNull { submission -> submission.localEditContext }
        if (context == null) {
          dispatch(messages.toList(), target)
        } else {
          withContext(context) { dispatch(messages.toList(), target) }
        }
      }
      submissions.forEach(Submission::complete)
      submissions.clear()
      messages.clear()
      target = null
    }
  }

  private val submissions = Channel<Submission>(Channel.UNLIMITED)

  init {
    scope.launch {
      try {
        for (first in submissions) {
          drainFrom(first)
        }
      } finally {
        val cancellation = CancellationException("Editor key binding coalescer stopped")
        submissions.close(cancellation)
        while (true) {
          val queued = submissions.tryReceive().getOrNull() ?: break
          queued.fail(cancellation)
        }
      }
    }
  }

  fun submit(
    binding: KeyBinding,
    clipboard: Clipboard,
    localEditContext: CoroutineContext? = null,
  ): Deferred<Unit> {
    val submission = Submission(binding, clipboard, localEditContext, CompletableDeferred())
    val result = submissions.trySend(submission)
    if (result.isClosed) {
      // Callers sit outside any coroutine (hardware key handlers), so a stopped
      // worker must surface as a failed completion, never a synchronous throw.
      submission.fail(
        result.exceptionOrNull() ?: CancellationException("Editor key binding coalescer stopped")
      )
    }
    return submission.completion
  }

  private suspend fun drainFrom(first: Submission) {
    val batch = PendingBatch()
    val claimed = mutableListOf<Submission>()
    var current: Submission? = first
    try {
      while (current != null) {
        claimed += current
        val binding = current.binding
        if (binding.coalescible) {
          batch.add(current, resolveMessages(binding, current.clipboard))
        } else {
          batch.flush()
          val messages = resolveMessages(binding, current.clipboard)
          if (current.localEditContext == null) {
            dispatch(messages, binding.bringIntoViewTarget)
          } else {
            withContext(current.localEditContext) {
              dispatch(messages, binding.bringIntoViewTarget)
            }
          }
          current.complete()
        }
        current = submissions.tryReceive().getOrNull()
      }
      batch.flush()
    } catch (error: Throwable) {
      claimed.forEach { it.fail(error) }
      // A quiesced editor cancels one dispatch; only scope cancellation stops the worker itself.
      if (error is CancellationException && !currentCoroutineContext().isActive) throw error
    }
  }
}
