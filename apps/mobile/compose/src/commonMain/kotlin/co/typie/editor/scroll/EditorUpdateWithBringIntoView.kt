package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.EditorRequestScope
import co.typie.editor.EditorUpdate
import co.typie.editor.beforePublish
import co.typie.editor.ffi.Message

internal interface EditorBringIntoViewUpdateScope : EditorRequestScope {
  fun bringIntoView(target: EditorBringIntoViewTarget)
}

private fun EditorRequestScope.applyBringIntoViewUpdate(
  block: EditorBringIntoViewUpdateScope.() -> Unit
): EditorBringIntoViewTarget? {
  val request = this
  var selectedTarget: EditorBringIntoViewTarget? = null
  block(
    object : EditorBringIntoViewUpdateScope {
      override fun enqueue(message: Message) {
        request.enqueue(message)
      }

      override fun bringIntoView(target: EditorBringIntoViewTarget) {
        selectedTarget = target
      }
    }
  )
  return selectedTarget
}

internal fun Editor.updateNowWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  block: EditorBringIntoViewUpdateScope.() -> Unit,
): EditorUpdate? = updateNow {
  applyBringIntoViewUpdate(block)?.let { target ->
    val request = bringIntoViewRequests.declare(target)
    beforePublish(
      block = { applied -> bringIntoViewRequests.bind(request, applied.revision) },
      onDiscard = { bringIntoViewRequests.discard(request) },
    )
  }
}

internal suspend fun Editor.updateWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  admit: () -> Boolean = { true },
  block: EditorBringIntoViewUpdateScope.() -> Unit,
): EditorUpdate? {
  var reveal: EditorBringIntoViewRequests.Request? = null
  val update =
    update(admit = admit) {
      applyBringIntoViewUpdate(block)?.let { target ->
        val request = bringIntoViewRequests.declare(target)
        reveal = request
        beforePublish(
          block = { applied -> bringIntoViewRequests.bind(request, applied.revision) },
          onDiscard = { bringIntoViewRequests.discard(request) },
        )
      }
    }
  if (update == null) {
    return null
  }
  reveal
    ?.takeIf { it.behavior == EditorBringIntoViewBehavior.Instant }
    ?.let { bringIntoViewRequests.awaitPresentation(it) }
  return update
}
