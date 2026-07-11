package co.typie.screen.editor.editor.subpane.comments

import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.StableSelection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.Underline
import co.typie.editor.ffi.UnderlineStyle

internal const val COMMENT_RANGE_GROUP = "comment"
internal const val ACTIVE_COMMENT_RANGE_GROUP = "comment-active"
internal const val COMMENT_COMPOSE_RANGE_GROUP = "__comment_compose__"
internal const val COMMENT_COMPOSE_RANGE_ID = "__comment_compose__"

internal suspend fun Editor.installCommentDecorations() {
  await { installCommentDecorations(this) }
}

internal suspend fun Editor.syncCommentRanges(
  selectionsById: Map<String, StableSelection>,
  activeId: String?,
  currentRanges: List<TrackedRange>,
) {
  val trackedRanges = currentRanges.commentRanges()
  val registeredIds = trackedRanges.mapTo(mutableSetOf()) { it.id }
  val desiredIds = selectionsById.keys
  val activeCommentId = activeId?.takeIf { it in desiredIds }

  await {
    (registeredIds - desiredIds).forEach { id ->
      enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id)))
    }

    selectionsById.forEach { (id, selection) ->
      if (id !in registeredIds) {
        enqueue(
          Message.TrackedRange(
            TrackedRangeOp.AddFrozen(
              id = id,
              group =
                if (id == activeCommentId) ACTIVE_COMMENT_RANGE_GROUP else COMMENT_RANGE_GROUP,
              selection = selection,
            )
          )
        )
      }
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
}

internal suspend fun Editor.setCommentComposeRange(selection: StableSelection?) {
  await {
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

internal val TrackedRangeEndpoints.isCommentRange: Boolean
  get() = group == COMMENT_RANGE_GROUP || group == ACTIVE_COMMENT_RANGE_GROUP

internal fun List<TrackedRangeEndpoints>.commentRangeEndpoints(): List<TrackedRangeEndpoints> =
  filter {
    it.isCommentRange
  }

private fun installCommentDecorations(scope: EditorScope) {
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
