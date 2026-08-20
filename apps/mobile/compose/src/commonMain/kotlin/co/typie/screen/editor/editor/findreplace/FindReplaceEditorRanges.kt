package co.typie.screen.editor.editor.findreplace

import co.typie.editor.Editor
import co.typie.editor.EditorRequestScope
import co.typie.editor.ffi.CommandOutcome
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.ViewOp
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.updateWithBringIntoView

internal const val SEARCH_MATCH_RANGE_GROUP = "search-match"
internal const val ACTIVE_SEARCH_MATCH_RANGE_GROUP = "search-match-active"

internal data class FindReplaceRangeRegistration(val id: String, val selection: Selection)

internal data class FindReplaceMatch(val id: String, val selection: Selection)

internal fun List<TrackedRange>.searchMatchRanges(): List<TrackedRange> = filter {
  it.group == SEARCH_MATCH_RANGE_GROUP || it.group == ACTIVE_SEARCH_MATCH_RANGE_GROUP
}

internal suspend fun Editor.installFindReplaceDecorations() {
  update {
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
  runCallback {
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)))
    enqueue(
      Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP))
    )
  }
}

internal suspend fun Editor.setFindReplaceRanges(
  items: List<FindReplaceRangeRegistration>,
  activeId: String?,
) {
  update { enqueueFindReplaceRangeReplacement(items = items, activeId = activeId) }
}

internal suspend fun Editor.setFindReplaceRangesAndReveal(
  items: List<FindReplaceRangeRegistration>,
  activeId: String?,
  bringIntoViewRequests: EditorBringIntoViewRequests,
) {
  updateWithBringIntoView(bringIntoViewRequests) {
    enqueueFindReplaceRangeReplacement(items = items, activeId = activeId)
    activeId?.let { id ->
      enqueue(Message.View(ViewOp.ExpandFoldsForTrackedRange(id)))
      bringIntoView(
        target = EditorBringIntoViewTarget.TrackedItem(id),
        policy = EditorBringIntoViewPolicy.Reveal,
        behavior = EditorBringIntoViewBehavior.Smooth,
      )
    }
  }
}

internal suspend fun Editor.setActiveFindReplaceRangeAndReveal(
  activeId: String,
  currentRanges: List<TrackedRange>,
  bringIntoViewRequests: EditorBringIntoViewRequests,
) {
  updateWithBringIntoView(bringIntoViewRequests) {
    enqueueActiveFindReplaceRange(activeId = activeId, currentRanges = currentRanges)
    enqueue(Message.View(ViewOp.ExpandFoldsForTrackedRange(activeId)))
    bringIntoView(
      target = EditorBringIntoViewTarget.TrackedItem(activeId),
      policy = EditorBringIntoViewPolicy.Reveal,
      behavior = EditorBringIntoViewBehavior.Smooth,
    )
  }
}

private fun EditorRequestScope.enqueueFindReplaceRangeReplacement(
  items: List<FindReplaceRangeRegistration>,
  activeId: String?,
) {
  enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SEARCH_MATCH_RANGE_GROUP)))
  enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)))
  items.forEach { item ->
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.Add(
          id = item.id,
          group =
            if (item.id == activeId) {
              ACTIVE_SEARCH_MATCH_RANGE_GROUP
            } else {
              SEARCH_MATCH_RANGE_GROUP
            },
          selection = item.selection,
        )
      )
    )
  }
}

private fun EditorRequestScope.enqueueActiveFindReplaceRange(
  activeId: String,
  currentRanges: List<TrackedRange>,
) {
  val searchRanges = currentRanges.searchMatchRanges()
  searchRanges
    .filter { it.group == ACTIVE_SEARCH_MATCH_RANGE_GROUP && it.id != activeId }
    .forEach { range ->
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.SetGroup(id = range.id, group = SEARCH_MATCH_RANGE_GROUP)
        )
      )
    }

  if (searchRanges.any { it.id == activeId && it.group != ACTIVE_SEARCH_MATCH_RANGE_GROUP }) {
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.SetGroup(id = activeId, group = ACTIVE_SEARCH_MATCH_RANGE_GROUP)
      )
    )
  }
}

internal suspend fun Editor.replaceFindReplaceRangeText(
  id: String,
  expectedText: String,
  replacement: String,
  admit: () -> Boolean = { true },
): Boolean =
  update(admit = admit) {
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.ReplaceText(
            id = id,
            expectedText = expectedText,
            replacement = replacement,
          )
        )
      )
      enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id)))
    }
    ?.commandOutcomes
    ?.none { it is CommandOutcome.Rejected } == true

internal suspend fun Editor.replaceAllFindReplaceRanges(
  matches: List<FindReplaceMatch>,
  expectedText: String,
  replacement: String,
  admit: () -> Boolean = { true },
): Boolean =
  update(admit = admit) {
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
    ?.commandOutcomes
    ?.none { it is CommandOutcome.Rejected } == true
