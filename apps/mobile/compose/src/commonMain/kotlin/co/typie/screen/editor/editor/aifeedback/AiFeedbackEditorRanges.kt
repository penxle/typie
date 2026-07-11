package co.typie.screen.editor.editor.aifeedback

import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import co.typie.editor.ffi.TrackedRangeOp

internal const val AI_FEEDBACK_RANGE_GROUP = "ai-feedback"
internal const val ACTIVE_AI_FEEDBACK_RANGE_GROUP = "ai-feedback-active"

internal data class AiFeedbackRangeRegistration(val id: String, val selection: Selection)

internal suspend fun Editor.installAiFeedbackDecorations() {
  await { installAiFeedbackDecorations(this) }
}

internal fun Editor.clearAiFeedbackRanges() {
  enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = AI_FEEDBACK_RANGE_GROUP)))
  enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_AI_FEEDBACK_RANGE_GROUP)))
}

internal suspend fun Editor.addAiFeedbackRange(item: AiFeedbackRangeRegistration) {
  await {
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.Add(
          id = item.id,
          group = AI_FEEDBACK_RANGE_GROUP,
          selection = item.selection,
        )
      )
    )
  }
}

internal suspend fun Editor.setActiveAiFeedbackRange(
  activeId: String?,
  currentRanges: List<TrackedRange>,
) {
  val aiFeedbackRanges = currentRanges.aiFeedbackRanges()
  await {
    aiFeedbackRanges
      .filter { it.group == ACTIVE_AI_FEEDBACK_RANGE_GROUP && it.id != activeId }
      .forEach { range ->
        enqueue(
          Message.TrackedRange(
            TrackedRangeOp.SetGroup(id = range.id, group = AI_FEEDBACK_RANGE_GROUP)
          )
        )
      }

    if (
      activeId != null &&
        aiFeedbackRanges.any { it.id == activeId && it.group != ACTIVE_AI_FEEDBACK_RANGE_GROUP }
    ) {
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.SetGroup(id = activeId, group = ACTIVE_AI_FEEDBACK_RANGE_GROUP)
        )
      )
    }
  }
}

internal suspend fun Editor.removeAiFeedbackRange(id: String) {
  await { enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id))) }
}

internal suspend fun Editor.removeAiFeedbackRanges(ids: Iterable<String>) {
  await { ids.forEach { id -> enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id))) } }
}

internal val TrackedRangeEndpoints.isAiFeedbackRange: Boolean
  get() = group == AI_FEEDBACK_RANGE_GROUP || group == ACTIVE_AI_FEEDBACK_RANGE_GROUP

internal fun List<TrackedRangeEndpoints>.aiFeedbackRangeEndpoints(): List<TrackedRangeEndpoints> =
  filter {
    it.isAiFeedbackRange
  }

internal fun List<TrackedRange>.aiFeedbackRanges(): List<TrackedRange> = filter {
  it.group == AI_FEEDBACK_RANGE_GROUP || it.group == ACTIVE_AI_FEEDBACK_RANGE_GROUP
}

private fun installAiFeedbackDecorations(scope: EditorScope) {
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = AI_FEEDBACK_RANGE_GROUP,
        style =
          DecorationStyle(
            background = "bg.blue",
            backgroundRadius = 2f,
            backgroundInset = 2f,
            underline = null,
          ),
        enabled = true,
        zIndex = 0,
      )
    )
  )
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = ACTIVE_AI_FEEDBACK_RANGE_GROUP,
        style =
          DecorationStyle(
            background = "bg.purple",
            backgroundRadius = 2f,
            backgroundInset = 2f,
            underline = null,
          ),
        enabled = true,
        zIndex = 1,
      )
    )
  )
}
