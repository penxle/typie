package co.typie.editor.scroll

import co.typie.editor.Editor
import co.typie.editor.EditorRequestScope
import co.typie.editor.EditorUpdate
import co.typie.editor.beforePublish
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ViewOp

internal interface EditorBringIntoViewUpdateScope : EditorRequestScope {
  fun bringIntoView(
    target: EditorBringIntoViewTarget,
    policy: EditorBringIntoViewPolicy,
    behavior: EditorBringIntoViewBehavior = EditorBringIntoViewBehavior.Instant,
  )
}

private data class EditorBringIntoViewUpdate(
  val target: EditorBringIntoViewTarget,
  val policy: EditorBringIntoViewPolicy,
  val behavior: EditorBringIntoViewBehavior,
)

private fun EditorRequestScope.applyBringIntoViewUpdate(
  block: EditorBringIntoViewUpdateScope.() -> Unit
): EditorBringIntoViewUpdate? {
  val request = this
  var selectedUpdate: EditorBringIntoViewUpdate? = null
  block(
    object : EditorBringIntoViewUpdateScope {
      override fun enqueue(message: Message) {
        request.enqueue(message)
      }

      override fun bringIntoView(
        target: EditorBringIntoViewTarget,
        policy: EditorBringIntoViewPolicy,
        behavior: EditorBringIntoViewBehavior,
      ) {
        selectedUpdate =
          EditorBringIntoViewUpdate(target = target, policy = policy, behavior = behavior)
      }
    }
  )
  return selectedUpdate
}

internal fun Editor.updateNowWithBringIntoView(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  block: EditorBringIntoViewUpdateScope.() -> Unit,
): EditorUpdate? = updateNow {
  applyBringIntoViewUpdate(block)?.let { reveal ->
    val request =
      bringIntoViewRequests.declare(
        target = reveal.target,
        policy = reveal.policy,
        behavior = reveal.behavior,
      )
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
      applyBringIntoViewUpdate(block)?.let { requested ->
        val request =
          bringIntoViewRequests.declare(
            target = requested.target,
            policy = requested.policy,
            behavior = requested.behavior,
          )
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

internal suspend fun Editor.revealTrackedItem(
  bringIntoViewRequests: EditorBringIntoViewRequests,
  id: String,
): EditorUpdate? =
  updateWithBringIntoView(bringIntoViewRequests) {
    enqueue(Message.View(ViewOp.ExpandFoldsForTrackedRange(id)))
    bringIntoView(
      target = EditorBringIntoViewTarget.TrackedItem(id),
      policy = EditorBringIntoViewPolicy.Reveal,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )
  }
