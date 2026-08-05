package co.typie.screen.editor.editor.subpane.comments

import co.touchlab.kermit.Logger
import co.typie.editor.Editor
import co.typie.editor.EditorRequestScope
import co.typie.editor.TryEnqueueResult
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.Underline
import co.typie.editor.ffi.UnderlineStyle
import co.typie.serialization.json
import io.sentry.kotlin.multiplatform.Sentry
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.decodeFromJsonElement

internal const val COMMENT_RANGE_GROUP = "comment"
internal const val ACTIVE_COMMENT_RANGE_GROUP = "comment-active"
internal const val COMMENT_COMPOSE_RANGE_GROUP = "__comment_compose__"
internal const val COMMENT_COMPOSE_RANGE_ID = "__comment_compose__"
internal val COMMENT_MEMBERSHIP_GROUPS = setOf(COMMENT_RANGE_GROUP, ACTIVE_COMMENT_RANGE_GROUP)

internal suspend fun Editor.installCommentDecorations() {
  update { installCommentDecorations(this) }
}

internal suspend fun Editor.syncCommentRanges(
  selectionsById: Map<String, JsonElement>,
  activeId: String?,
  currentRanges: List<TrackedRange>,
) {
  val trackedRanges = currentRanges.commentRanges()
  val registeredIds = trackedRanges.mapTo(mutableSetOf()) { it.id }
  val desiredIds = selectionsById.keys
  val activeCommentId = activeId?.takeIf { it in desiredIds }

  update {
    (registeredIds - desiredIds).forEach { id ->
      enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id)))
    }

    trackedRanges
      .filter {
        it.id in desiredIds && it.group == ACTIVE_COMMENT_RANGE_GROUP && it.id != activeCommentId
      }
      .forEach { range ->
        enqueue(
          Message.TrackedRange(TrackedRangeOp.SetGroup(id = range.id, group = COMMENT_RANGE_GROUP))
        )
      }

    if (
      activeCommentId != null &&
        trackedRanges.any { it.id == activeCommentId && it.group != ACTIVE_COMMENT_RANGE_GROUP }
    ) {
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.SetGroup(id = activeCommentId, group = ACTIVE_COMMENT_RANGE_GROUP)
        )
      )
    }
  }

  selectionsById.forEach { (id, selection) ->
    if (id !in registeredIds) {
      addFrozenComment(
        id = id,
        group = if (id == activeCommentId) ACTIVE_COMMENT_RANGE_GROUP else COMMENT_RANGE_GROUP,
        selection = selection,
      )
    }
  }
}

internal fun Editor.addFrozenComment(id: String, group: String, selection: JsonElement) {
  val result = tryEnqueue {
    Message.TrackedRange(
      TrackedRangeOp.AddFrozen(
        id = id,
        group = group,
        selection = json.decodeFromJsonElement<StableSelection>(selection),
      )
    )
  }
  if (result is TryEnqueueResult.Failed) {
    reportError(result.error, "Failed to add frozen comment: $id")
  }
}

internal suspend fun Editor.setCommentComposeRange(selection: StableSelection?) {
  update {
    enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = COMMENT_COMPOSE_RANGE_ID)))
    if (selection != null) {
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.AddFrozen(
            id = COMMENT_COMPOSE_RANGE_ID,
            group = COMMENT_COMPOSE_RANGE_GROUP,
            selection = selection,
          )
        )
      )
    }
  }
}

internal fun List<TrackedRange>.commentRanges(): List<TrackedRange> = filter {
  it.group == COMMENT_RANGE_GROUP || it.group == ACTIVE_COMMENT_RANGE_GROUP
}

private fun installCommentDecorations(scope: EditorRequestScope) {
  val underline = Underline(color = "text.amber", style = UnderlineStyle.Solid, thickness = 2f)
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = COMMENT_RANGE_GROUP,
        style =
          DecorationStyle(
            background = "ui.comment-highlight",
            backgroundRadius = 2f,
            backgroundInset = 2f,
            underline = underline,
          ),
        enabled = true,
        zIndex = 0,
      )
    )
  )
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = ACTIVE_COMMENT_RANGE_GROUP,
        style =
          DecorationStyle(
            background = "ui.comment-highlight-active",
            backgroundRadius = 2f,
            backgroundInset = 2f,
            underline = underline,
          ),
        enabled = true,
        zIndex = 1,
      )
    )
  )
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = COMMENT_COMPOSE_RANGE_GROUP,
        style =
          DecorationStyle(
            background = "ui.comment-highlight-active",
            backgroundRadius = 2f,
            backgroundInset = 2f,
            underline = underline,
          ),
        enabled = true,
        zIndex = 2,
      )
    )
  )
}

private fun reportError(error: Throwable, message: String) {
  Logger.e(error) { message }
  runCatching { Sentry.captureException(error) }
}
