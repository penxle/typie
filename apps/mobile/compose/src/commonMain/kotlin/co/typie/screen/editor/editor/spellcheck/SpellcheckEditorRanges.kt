package co.typie.screen.editor.editor.spellcheck

import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ProseRangeInstallOutcome
import co.typie.editor.ffi.ProseTrackedRangeRegistration
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.Underline
import co.typie.editor.ffi.UnderlineStyle

internal const val SPELLCHECK_RANGE_GROUP = "spellcheck"
internal const val ACTIVE_SPELLCHECK_RANGE_GROUP = "spellcheck-active"

internal enum class SpellcheckRangeInstallResult {
  Ready,
  StaleCurrent,
  Superseded,
}

internal data class FailedSpellcheckRange(val index: Int, val start: Int, val end: Int)

internal class SpellcheckRangeInstallException(
  val rawResultCount: Int,
  val failedRanges: List<FailedSpellcheckRange>,
  val invalidRequestCategory: String? = null,
) :
  IllegalStateException(
    if (invalidRequestCategory == null) {
      "Spellcheck range mapping failed for ${failedRanges.size} of $rawResultCount results"
    } else {
      "Spellcheck range request was rejected: $invalidRequestCategory"
    }
  )

internal suspend fun Editor.installSpellcheckDecorations() {
  await { installSpellcheckDecorations(this) }
}

internal suspend fun Editor.clearSpellcheckRanges(admit: () -> Boolean = { true }): Boolean =
  clearTrackedRangeGroups(
    groups = listOf(SPELLCHECK_RANGE_GROUP, ACTIVE_SPELLCHECK_RANGE_GROUP),
    admit = admit,
  )

internal suspend fun Editor.installSpellcheckRangesFromProse(
  expectedText: String,
  items: List<RawSpellcheckResult>,
  isCurrent: () -> Boolean,
): SpellcheckRangeInstallResult {
  val ranges = items.mapIndexed { index, item ->
    ProseTrackedRangeRegistration(
      id = item.id,
      group = if (index == 0) ACTIVE_SPELLCHECK_RANGE_GROUP else SPELLCHECK_RANGE_GROUP,
      start = item.start,
      end = item.end,
    )
  }
  val outcome =
    replaceTrackedRangeGroupsFromProse(
      expectedText = expectedText,
      groups = listOf(SPELLCHECK_RANGE_GROUP, ACTIVE_SPELLCHECK_RANGE_GROUP),
      ranges = ranges,
      isCurrent = isCurrent,
    ) ?: return SpellcheckRangeInstallResult.Superseded

  return when (outcome) {
    ProseRangeInstallOutcome.Applied -> SpellcheckRangeInstallResult.Ready
    ProseRangeInstallOutcome.TextMismatch -> SpellcheckRangeInstallResult.StaleCurrent
    ProseRangeInstallOutcome.InvalidRequest ->
      throw SpellcheckRangeInstallException(
        rawResultCount = items.size,
        failedRanges = emptyList(),
        invalidRequestCategory = "core_rejected_request",
      )
    is ProseRangeInstallOutcome.InvalidRanges ->
      throw SpellcheckRangeInstallException(
        rawResultCount = items.size,
        failedRanges =
          outcome.indices.mapNotNull { index ->
            items.getOrNull(index)?.let { item ->
              FailedSpellcheckRange(index = index, start = item.start, end = item.end)
            }
          },
      )
  }
}

internal suspend fun Editor.setActiveSpellcheckRange(
  activeId: String?,
  currentRanges: List<TrackedRange>,
) {
  val spellcheckRanges = currentRanges.spellcheckRanges()
  await {
    spellcheckRanges
      .filter { it.group == ACTIVE_SPELLCHECK_RANGE_GROUP && it.id != activeId }
      .forEach { range ->
        enqueue(
          Message.TrackedRange(
            TrackedRangeOp.SetGroup(id = range.id, group = SPELLCHECK_RANGE_GROUP)
          )
        )
      }

    if (
      activeId != null &&
        spellcheckRanges.any { it.id == activeId && it.group != ACTIVE_SPELLCHECK_RANGE_GROUP }
    ) {
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.SetGroup(id = activeId, group = ACTIVE_SPELLCHECK_RANGE_GROUP)
        )
      )
    }
  }
}

internal suspend fun Editor.removeSpellcheckRange(id: String) {
  await { enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id))) }
}

internal suspend fun Editor.removeSpellcheckRanges(ids: Iterable<String>) {
  await { ids.forEach { id -> enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id))) } }
}

internal suspend fun Editor.replaceSpellcheckRangeText(
  id: String,
  expectedText: String,
  replacement: String,
) {
  await {
    enqueue(
      Message.TrackedRange(
        TrackedRangeOp.ReplaceText(id = id, expectedText = expectedText, replacement = replacement)
      )
    )
    enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id)))
  }
}

internal val TrackedRangeEndpoints.isSpellcheckRange: Boolean
  get() = group == SPELLCHECK_RANGE_GROUP || group == ACTIVE_SPELLCHECK_RANGE_GROUP

internal fun List<TrackedRangeEndpoints>.spellcheckRangeEndpoints(): List<TrackedRangeEndpoints> =
  filter {
    it.isSpellcheckRange
  }

internal fun List<TrackedRange>.spellcheckRanges(): List<TrackedRange> = filter {
  it.group == SPELLCHECK_RANGE_GROUP || it.group == ACTIVE_SPELLCHECK_RANGE_GROUP
}

private fun installSpellcheckDecorations(scope: EditorScope) {
  val underline = Underline(color = "text.red", style = UnderlineStyle.Wavy, thickness = 1.5f)
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = SPELLCHECK_RANGE_GROUP,
        style = DecorationStyle(background = null, underline = underline),
        enabled = true,
        zIndex = 0,
      )
    )
  )
  scope.enqueue(
    Message.TrackedRange(
      TrackedRangeOp.SetGroupDecoration(
        group = ACTIVE_SPELLCHECK_RANGE_GROUP,
        style =
          DecorationStyle(
            background = "bg.red",
            backgroundRadius = 2f,
            backgroundInset = 1f,
            underline = underline,
          ),
        enabled = true,
        zIndex = 1,
      )
    )
  )
}
