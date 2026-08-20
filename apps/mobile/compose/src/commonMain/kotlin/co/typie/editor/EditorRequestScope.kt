package co.typie.editor

import co.typie.editor.ffi.Message

interface EditorRequestScope {
  fun enqueue(message: Message)
}

internal class CollectedEditorRequest : EditorRequestScope {
  val messages = mutableListOf<Message>()
  private var beforePublish: ((EditorUpdate) -> Unit)? = null
  private var discardBeforePublish: (() -> Unit)? = null

  override fun enqueue(message: Message) {
    messages += message
  }

  fun beforePublish(block: (EditorUpdate) -> Unit, onDiscard: () -> Unit) {
    check(beforePublish == null) { "beforePublish may only be registered once" }
    beforePublish = block
    discardBeforePublish = onDiscard
  }

  fun runBeforePublish(update: EditorUpdate) {
    val block = beforePublish
    val onDiscard = discardBeforePublish
    beforePublish = null
    discardBeforePublish = null
    try {
      block?.invoke(update)
    } catch (error: Throwable) {
      runCleanupPreservingFailure(error) { onDiscard?.invoke() }
      throw error
    }
  }

  fun discard() {
    val onDiscard = discardBeforePublish
    beforePublish = null
    discardBeforePublish = null
    onDiscard?.invoke()
  }

  fun discardAfterFailure(error: Throwable) {
    runCleanupPreservingFailure(error, ::discard)
  }
}

private inline fun runCleanupPreservingFailure(error: Throwable, cleanup: () -> Unit) {
  val failure = error.unwrapEditorFailureSignal()
  try {
    cleanup()
  } catch (cleanupFailure: Throwable) {
    if (cleanupFailure !== failure) failure.addSuppressed(cleanupFailure)
  }
}

internal fun EditorRequestScope.beforePublish(
  block: (EditorUpdate) -> Unit,
  onDiscard: () -> Unit,
) {
  val request =
    this as? CollectedEditorRequest ?: error("beforePublish is only valid during update admission")
  request.beforePublish(block, onDiscard)
}
