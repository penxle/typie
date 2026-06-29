package co.typie.screen.editor.editor.spellcheck

import co.typie.editor.Editor
import co.typie.editor.EditorScope
import co.typie.editor.ffi.DecorationStyle
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TrackedRange
import co.typie.editor.ffi.TrackedRangeEndpoints
import co.typie.editor.ffi.TrackedRangeOp
import co.typie.editor.ffi.Underline
import co.typie.editor.ffi.UnderlineStyle

internal const val SPELLCHECK_RANGE_GROUP = "spellcheck"
internal const val ACTIVE_SPELLCHECK_RANGE_GROUP = "spellcheck-active"

internal data class SpellcheckRangeRegistration(val id: String, val selection: Selection)

internal fun Editor.installSpellcheckDecorations() {
  sync { installSpellcheckDecorations(this) }
}

internal fun Editor.clearSpellcheckRanges() {
  sync {
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SPELLCHECK_RANGE_GROUP)))
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SPELLCHECK_RANGE_GROUP)))
  }
}

internal fun Editor.setSpellcheckRanges(items: List<SpellcheckRangeRegistration>) {
  sync {
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = SPELLCHECK_RANGE_GROUP)))
    enqueue(Message.TrackedRange(TrackedRangeOp.ClearGroup(group = ACTIVE_SPELLCHECK_RANGE_GROUP)))
    items.forEach { item ->
      enqueue(
        Message.TrackedRange(
          TrackedRangeOp.Add(
            id = item.id,
            group = SPELLCHECK_RANGE_GROUP,
            selection = item.selection,
          )
        )
      )
    }
  }
}

internal fun Editor.setActiveSpellcheckRange(activeId: String?, currentRanges: List<TrackedRange>) {
  val spellcheckRanges = currentRanges.spellcheckRanges()
  sync {
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

internal fun Editor.removeSpellcheckRange(id: String) {
  sync { enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id))) }
}

internal fun Editor.removeSpellcheckRanges(ids: Iterable<String>) {
  sync { ids.forEach { id -> enqueue(Message.TrackedRange(TrackedRangeOp.Remove(id = id))) } }
}

internal fun Editor.replaceSpellcheckRangeText(
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
