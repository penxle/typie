package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.ffi.Message

internal interface EditorBringIntoViewAwaitScope : EditorScope {
  fun beforeCommit(block: EditorBringIntoViewCommitScope.() -> Unit)
}

internal interface EditorBringIntoViewSyncScope : EditorScope {
  fun beforeCommit(block: EditorBringIntoViewCommitScope.() -> Unit)
}

internal interface EditorBringIntoViewCommitScope {
  fun bringIntoView(target: EditorBringIntoViewTarget)
}

internal fun Editor.syncWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  block: EditorBringIntoViewSyncScope.() -> Unit,
) {
  val beforeCommitBlocks = mutableListOf<EditorBringIntoViewCommitScope.() -> Unit>()

  sync(
    beforeCommit = { snapshot ->
      val commitScope =
        object : EditorBringIntoViewCommitScope {
          override fun bringIntoView(target: EditorBringIntoViewTarget) {
            bringIntoViewRequests.requestForVersion(target = target, version = snapshot.version)
          }
        }

      beforeCommitBlocks.forEach { block -> block(commitScope) }
    }
  ) {
    val editorScope = this
    val syncScope =
      object : EditorBringIntoViewSyncScope {
        override fun enqueue(message: Message) {
          editorScope.enqueue(message)
        }

        override fun beforeCommit(block: EditorBringIntoViewCommitScope.() -> Unit) {
          beforeCommitBlocks += block
        }
      }

    syncScope.block()
  }
}

internal suspend fun Editor.awaitWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  block: EditorBringIntoViewAwaitScope.() -> Unit,
) {
  val beforeCommitBlocks = mutableListOf<EditorBringIntoViewCommitScope.() -> Unit>()

  await(
    beforeCommit = { snapshot ->
      val commitScope =
        object : EditorBringIntoViewCommitScope {
          override fun bringIntoView(target: EditorBringIntoViewTarget) {
            bringIntoViewRequests.requestForVersion(target = target, version = snapshot.version)
          }
        }

      beforeCommitBlocks.forEach { block -> block(commitScope) }
    }
  ) {
    val editorScope = this
    val awaitScope =
      object : EditorBringIntoViewAwaitScope {
        override fun enqueue(message: Message) {
          editorScope.enqueue(message)
        }

        override fun beforeCommit(block: EditorBringIntoViewCommitScope.() -> Unit) {
          beforeCommitBlocks += block
        }
      }

    awaitScope.block()
  }
}
