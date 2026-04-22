package co.typie.screen.editor.editor.scroll

import co.typie.screen.editor.editor.layout.EditorVisibleArea
import kotlin.math.max
import kotlin.math.min

internal const val CursorVisibleMargin = 60f
private const val TypewriterBandHeight = 56f
private const val TypewriterBandFraction = 0.42f

internal enum class EditorScrollMode {
  KeepCursorVisible,
  Typewriter,
}

internal data class EditorScrollRange(val top: Float = 0f, val bottom: Float = 0f) {
  val isValid: Boolean
    get() = bottom > top
}

internal data class EditorScrollPolicy(
  val mode: EditorScrollMode,
  val keepVisibleRange: EditorScrollRange,
  val typewriterRange: EditorScrollRange,
  val typewriterBottomPadding: Float,
) {
  val activeRange: EditorScrollRange
    get() =
      when (mode) {
        EditorScrollMode.KeepCursorVisible -> keepVisibleRange
        EditorScrollMode.Typewriter -> typewriterRange
      }
}

internal fun resolveEditorScrollPolicy(
  visibleArea: EditorVisibleArea,
  defaultBottomPadding: Float,
): EditorScrollPolicy {
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  val typewriterRange = resolveTypewriterRange(keepVisibleRange)

  return EditorScrollPolicy(
    mode = EditorScrollMode.KeepCursorVisible, // TODO(editor-parity): Select the active scroll mode
    // from editor/document settings once typewriter mode
    // is wired through the KMP screen state.
    keepVisibleRange = keepVisibleRange,
    typewriterRange = typewriterRange,
    typewriterBottomPadding =
      resolveTypewriterBottomPadding(
        defaultBottomPadding = defaultBottomPadding,
        keepVisibleRange = keepVisibleRange,
        typewriterRange = typewriterRange,
      ),
  )
}

internal fun resolveEditorScrollTarget(
  currentScroll: Float,
  cursorTopInContent: Float,
  cursorBottomInContent: Float,
  range: EditorScrollRange,
): Float? {
  if (!range.isValid) {
    return null
  }

  val cursorTopInViewport = cursorTopInContent - currentScroll
  val cursorBottomInViewport = cursorBottomInContent - currentScroll

  return when {
    cursorBottomInViewport > range.bottom -> cursorBottomInContent - range.bottom
    cursorTopInViewport < range.top -> cursorTopInContent - range.top
    else -> null
  }
}

internal fun resolveKeepVisibleScrollTarget(
  currentScroll: Float,
  cursorTopInContent: Float,
  cursorBottomInContent: Float,
  visibleArea: EditorVisibleArea,
): Float? {
  return resolveEditorScrollTarget(
    currentScroll = currentScroll,
    cursorTopInContent = cursorTopInContent,
    cursorBottomInContent = cursorBottomInContent,
    range = resolveKeepVisibleRange(visibleArea),
  )
}

private fun resolveKeepVisibleRange(visibleArea: EditorVisibleArea): EditorScrollRange {
  val top = visibleArea.visibleViewportTop + CursorVisibleMargin
  val bottom = visibleArea.visibleViewportBottom - CursorVisibleMargin
  return if (bottom <= top) {
    EditorScrollRange()
  } else {
    EditorScrollRange(top = top, bottom = bottom)
  }
}

private fun resolveTypewriterRange(keepVisibleRange: EditorScrollRange): EditorScrollRange {
  if (!keepVisibleRange.isValid) {
    return EditorScrollRange()
  }

  val availableHeight = keepVisibleRange.bottom - keepVisibleRange.top
  val center = keepVisibleRange.top + availableHeight * TypewriterBandFraction
  val halfBand = min(TypewriterBandHeight / 2f, availableHeight / 2f)

  return EditorScrollRange(
    top = max(keepVisibleRange.top, center - halfBand),
    bottom = min(keepVisibleRange.bottom, center + halfBand),
  )
}

private fun resolveTypewriterBottomPadding(
  defaultBottomPadding: Float,
  keepVisibleRange: EditorScrollRange,
  typewriterRange: EditorScrollRange,
): Float {
  if (!keepVisibleRange.isValid || !typewriterRange.isValid) {
    return defaultBottomPadding
  }

  val typewriterTarget = (typewriterRange.top + typewriterRange.bottom) / 2f
  // TODO(editor-parity): Verify this padding formula against Flutter once the final
  // typewriter target band and viewport metrics are ported 1:1.
  return max(defaultBottomPadding, keepVisibleRange.bottom - typewriterTarget)
}
