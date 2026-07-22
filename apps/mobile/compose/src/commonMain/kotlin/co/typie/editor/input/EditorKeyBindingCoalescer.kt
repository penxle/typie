package co.typie.editor.input

import co.typie.editor.EditorKeyBindingAction
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
  private val resolveMessages:
    suspend (EditorKeyBindingAction.Messages, Clipboard) -> List<Message>,
  private val dispatch: suspend (List<Message>, EditorBringIntoViewTarget?) -> Unit,
) {
  private sealed interface Submission {
    val completion: CompletableDeferred<Unit>
  }

  private data class BindingSubmission(
    val binding: KeyBinding,
    val clipboard: Clipboard,
    val localEditContext: CoroutineContext?,
    override val completion: CompletableDeferred<Unit>,
  ) : Submission

  private data class OrderedActionSubmission(
    val action: suspend () -> Unit,
    override val completion: CompletableDeferred<Unit>,
  ) : Submission

  private fun Submission.complete() {
    completion.complete(Unit)
  }

  private fun Submission.fail(error: Throwable) {
    completion.completeExceptionally(error)
  }

  private inner class PendingBatch {
    private val submissions = mutableListOf<BindingSubmission>()
    private val messages = mutableListOf<Message>()
    private var target: EditorBringIntoViewTarget? = null

    fun add(submission: BindingSubmission, resolvedMessages: List<Message>) {
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
      submissions.forEach { it.complete() }
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
    val submission = BindingSubmission(binding, clipboard, localEditContext, CompletableDeferred())
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

  fun submitOrdered(action: suspend () -> Unit): Deferred<Unit> {
    val submission = OrderedActionSubmission(action, CompletableDeferred())
    val result = submissions.trySend(submission)
    if (result.isClosed) {
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
        when (val submission = current) {
          is BindingSubmission -> {
            val binding = submission.binding
            val action =
              binding.action as? EditorKeyBindingAction.Messages
                ?: error("Only message key bindings may be submitted to the coalescer")
            if (action.coalescible) {
              batch.add(submission, resolveMessages(action, submission.clipboard))
            } else {
              batch.flush()
              val messages = resolveMessages(action, submission.clipboard)
              if (submission.localEditContext == null) {
                dispatch(messages, binding.bringIntoViewTarget)
              } else {
                withContext(submission.localEditContext) {
                  dispatch(messages, binding.bringIntoViewTarget)
                }
              }
              submission.complete()
            }
          }
          is OrderedActionSubmission -> {
            batch.flush()
            submission.action()
            submission.complete()
          }
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
