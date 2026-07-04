package co.typie.screen.editor.editor.findreplace

import co.typie.editor.Editor
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.toPageRectsTarget

internal const val SEARCH_MATCH_RANGE_GROUP = "search-match"
internal const val ACTIVE_SEARCH_MATCH_RANGE_GROUP = "search-match-active"

internal data class FindReplaceRangeRegistration(val id: String, val selection: Selection)

internal data class FindReplaceMatch(val id: String, val selection: Selection)

internal fun List<TrackedRange>.searchMatchRanges(): List<TrackedRange> = filter {
  it.group == SEARCH_MATCH_RANGE_GROUP || it.group == ACTIVE_SEARCH_MATCH_RANGE_GROUP
}

internal fun Editor.installFindReplaceDecorations() {
  sync {
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.SetGroupDecoration(
          group = SEARCH_MATCH_RANGE_GROUP,
          style =
            DecorationStyle(
              background = "ui.search-match",
              backgroundRadius = 2f,
              backgroundInset = 2f,
              underline = null,
            ),
          enabled = true,
          zIndex = 0,
        )
      )
    )
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.SetGroupDecoration(
          group = ACTIVE_SEARCH_MATCH_RANGE_GROUP,
          style =
            DecorationStyle(
              background = "ui.search-match-active",
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
}

internal fun Editor.clearFindReplaceRanges() {
  sync {
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)))
    enqueue(
      Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP))
    )
  }
}

internal fun Editor.setFindReplaceRanges(items: List<FindReplaceRangeRegistration>) {
  sync {
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)))
    enqueue(
      Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP))
    )
    items.forEach { item ->
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.Add(
            id = item.id,
            group = SEARCH_MATCH_RANGE_GROUP,
            selection = item.selection,
          )
        )
      )
    }
  }
}

internal fun Editor.setActiveFindReplaceRange(
  activeId: String?,
  currentRanges: List<TrackedRange>,
) {
  val searchRanges = currentRanges.searchMatchRanges()
  sync {
    searchRanges
      .filter { it.group == ACTIVE_SEARCH_MATCH_RANGE_GROUP && it.id != activeId }
      .forEach { range ->
        enqueue(
          Message.TrackedRange(
            TrackedRangeOp.SetGroup(id = range.id, group = SEARCH_MATCH_RANGE_GROUP)
          )
        )
      }

    if (
      activeId != null &&
        searchRanges.any { it.id == activeId && it.group != ACTIVE_SEARCH_MATCH_RANGE_GROUP }
    ) {
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.SetGroup(id = activeId, group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)
        )
      )
    }
  }
}

internal fun Editor.replaceFindReplaceRangeText(
  id: String,
  expectedText: String,
  replacement: String,
) {
  sync {
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.ReplaceText(id = id, expectedText = expectedText, replacement = replacement)
      )
    )
    enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id)))
  }
}

internal fun Editor.replaceAllFindReplaceRanges(
  matches: List<FindReplaceMatch>,
  expectedText: String,
  replacement: String,
) {
  sync {
    matches.forEach { match ->
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.ReplaceText(
            id = match.id,
            expectedText = expectedText,
            replacement = replacement,
          )
        )
      )
    }
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)))
    enqueue(
      Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP))
    )
  }
}

internal fun List<TrackedRange>.searchMatchScrollTarget(id: String?): EditorBringIntoViewTarget? {
  if (id == null) return null
  return searchMatchRanges().firstOrNull { it.id == id }?.rects?.toPageRectsTarget()
}
