package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.EditorRequestScope
import co.typie.editor.EditorState
import co.typie.editor.ffi.Message

internal interface EditorBringIntoViewUpdateScope : EditorRequestScope {
  fun afterApplied(block: EditorBringIntoViewAppliedScope.() -> Unit)
}

internal interface EditorBringIntoViewAppliedScope {
  fun bringIntoView(target: EditorBringIntoViewTarget)
}

internal fun Editor.updateNowWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  block: EditorBringIntoViewUpdateScope.() -> Unit,
): EditorState? {
  val afterAppliedBlocks = mutableListOf<EditorBringIntoViewAppliedScope.() -> Unit>()
  val update =
    updateNow {
      val editorScope = this
      val updateScope =
        object : EditorBringIntoViewUpdateScope {
          override fun enqueue(message: Message) {
            editorScope.enqueue(message)
          }

          override fun afterApplied(block: EditorBringIntoViewAppliedScope.() -> Unit) {
            afterAppliedBlocks += block
          }
        }

      updateScope.block()
    } ?: return null

  val appliedScope =
    object : EditorBringIntoViewAppliedScope {
      override fun bringIntoView(target: EditorBringIntoViewTarget) {
        // EditorView activates this request only from a matching published snapshot, after
        // Compose has received the matching geometry.
        bringIntoViewRequests.requestForVersion(target = target, version = update.revision)
      }
    }
  afterAppliedBlocks.forEach { block -> block(appliedScope) }
  return update.snapshot
}

internal suspend fun Editor.updateWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  admit: () -> Boolean = { true },
  block: EditorBringIntoViewUpdateScope.() -> Unit,
): EditorState? {
  val afterAppliedBlocks = mutableListOf<EditorBringIntoViewAppliedScope.() -> Unit>()
  val update =
    update(
      admit = admit,
      afterApplied = { applied ->
        val appliedScope =
          object : EditorBringIntoViewAppliedScope {
            override fun bringIntoView(target: EditorBringIntoViewTarget) {
              bringIntoViewRequests.requestForVersion(target = target, version = applied.revision)
            }
          }
        afterAppliedBlocks.forEach { block -> block(appliedScope) }
      },
    ) {
      val editorScope = this
      val updateScope =
        object : EditorBringIntoViewUpdateScope {
          override fun enqueue(message: Message) {
            editorScope.enqueue(message)
          }

          override fun afterApplied(block: EditorBringIntoViewAppliedScope.() -> Unit) {
            afterAppliedBlocks += block
          }
        }

      updateScope.block()
    } ?: return null
  return update.snapshot
}
