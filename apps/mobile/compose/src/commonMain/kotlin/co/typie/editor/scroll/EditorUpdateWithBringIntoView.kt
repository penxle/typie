package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.EditorRequestScope
import co.typie.editor.EditorUpdate
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
): EditorUpdate? {
  var selectedTarget: EditorBringIntoViewTarget? = null
  return updateNow(
    admit = { true },
    beforePublish = { applied ->
      selectedTarget?.let { target ->
        bringIntoViewRequests.requestForVersion(target = target, version = applied.revision)
      }
    },
  ) {
    selectedTarget = applyBringIntoViewUpdate(block)
  }
}

internal suspend fun Editor.updateWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  admit: () -> Boolean = { true },
  block: EditorBringIntoViewUpdateScope.() -> Unit,
): EditorUpdate? {
  var selectedTarget: EditorBringIntoViewTarget? = null
  return update(
    admit = admit,
    beforePublish = { applied ->
      selectedTarget?.let { target ->
        bringIntoViewRequests.requestForVersion(target = target, version = applied.revision)
      }
    },
  ) {
    selectedTarget = applyBringIntoViewUpdate(block)
  }
}
