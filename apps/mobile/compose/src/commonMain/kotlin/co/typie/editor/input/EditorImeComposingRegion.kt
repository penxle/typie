package co.typie.editor.input

import co.typie.editor.ffi.Ime

// A composing region is the preedit at the cursor; keyboards only ever place
// it on text they just composed or the word under the cursor. A region that
// projects far from both the selection and the active composition can only
// come from a broken coordinate base on the keyboard side (e.g. offsets
// cached across a window re-anchor), so it clears the composition instead of
// anchoring text thousands of characters away.
private const val COMPOSING_REGION_REACH_SLACK = 64

internal sealed interface ComposingRegionDecision {
  data class Set(val start: Int, val end: Int) : ComposingRegionDecision

  data object Clear : ComposingRegionDecision
}

internal fun resolveComposingRegion(ime: Ime?, start: Int, end: Int): ComposingRegionDecision {
  // AOSP parity: reversed ranges swap and a zero-length region clears the
  // composition (stock editors discard zero-length composing spans).
  if (start == end) {
    return ComposingRegionDecision.Clear
  }
  if (ime == null) {
    return ComposingRegionDecision.Clear
  }
  val flatStart = ime.projectWindowUtf16Index(minOf(start, end))
  val flatEnd = ime.projectWindowUtf16Index(maxOf(start, end))
  if (!ime.withinComposingReach(flatStart, flatEnd)) {
    return ComposingRegionDecision.Clear
  }
  return ComposingRegionDecision.Set(flatStart, flatEnd)
}

private fun Ime.withinComposingReach(start: Int, end: Int): Boolean =
  listOfNotNull(selection, composing).any { range ->
    start <= range.end + COMPOSING_REGION_REACH_SLACK &&
      end >= range.start - COMPOSING_REGION_REACH_SLACK
  }
