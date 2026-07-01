package co.typie.editor.scroll

import co.typie.editor.VerticalSpan
import kotlin.math.abs
import kotlin.math.max

internal const val CursorVisibleMargin = 60f
private const val TypewriterMinBottomPadding = 48f

internal enum class EditorAutoScrollMode {
  KeepCursorVisible,
  Typewriter,
}

internal data class EditorAutoScrollPolicy(
  val mode: EditorAutoScrollMode,
  val typewriterPosition: Float,
  val keepVisibleRange: VerticalSpan,
  val targetTop: Float?,
  val targetLineHeight: Float,
  val bottomSpacerHeight: Float,
) {
  val targetBottom: Float?
    get() = targetTop?.plus(targetLineHeight)
}

internal fun resolveEditorAutoScrollPolicy(
  visibleArea: EditorVisibleArea,
  bottomSpacerVisibleArea: EditorVisibleArea = visibleArea,
  baseBottomSpace: Float = 0f,
  distanceToPagesBottom: Float? = null,
  pageBottomRevealSpacerHeight: Float = 0f,
  typewriterEnabled: Boolean = false,
  typewriterPosition: Float = 0.5f,
  targetLineHeight: Float = 0f,
): EditorAutoScrollPolicy {
  val resolvedTypewriterPosition = typewriterPosition.coerceIn(0f, 1f)
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  val resolvedTargetLineHeight = targetLineHeight.coerceAtLeast(0f)
  val targetTop =
    resolveScrollTargetTop(
      visibleArea = visibleArea,
      position = resolvedTypewriterPosition,
      targetHeight = resolvedTargetLineHeight,
    )
  val keepVisibleBottomSpacerHeight =
    resolveKeepVisibleBottomSpacerHeight(
      visibleArea = bottomSpacerVisibleArea,
      baseBottomSpace = baseBottomSpace,
    )
  val autoScrollPolicyBottomSpacerHeight =
    if (typewriterEnabled) {
      resolveTypewriterBottomSpacerHeight(
        visibleArea = bottomSpacerVisibleArea,
        baseBottomSpace = baseBottomSpace,
        distanceToPagesBottom = distanceToPagesBottom,
        position = resolvedTypewriterPosition,
        targetLineHeight = resolvedTargetLineHeight,
      )
    } else {
      keepVisibleBottomSpacerHeight
    }

  return EditorAutoScrollPolicy(
    mode =
      if (typewriterEnabled) EditorAutoScrollMode.Typewriter
      else EditorAutoScrollMode.KeepCursorVisible,
    typewriterPosition = resolvedTypewriterPosition,
    keepVisibleRange = keepVisibleRange,
    targetTop = targetTop,
    targetLineHeight = resolvedTargetLineHeight,
    bottomSpacerHeight =
      max(autoScrollPolicyBottomSpacerHeight, pageBottomRevealSpacerHeight.coerceAtLeast(0f)),
  )
}

internal fun resolveEditorScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  range: VerticalSpan,
): Float? {
  if (!range.isValid) {
    return null
  }

  val targetTopInViewport = targetTopInContent - currentScroll
  val targetBottomInViewport = targetBottomInContent - currentScroll

  return when {
    targetBottomInViewport > range.bottom -> targetBottomInContent - range.bottom
    targetTopInViewport < range.top -> targetTopInContent - range.top
    else -> null
  }
}

internal fun resolveKeepVisibleScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
): Float? {
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  if (!keepVisibleRange.isValid) {
    return resolveCenteredVisibleScrollOffset(
      currentScroll = currentScroll,
      targetTopInContent = targetTopInContent,
      targetBottomInContent = targetBottomInContent,
      visibleArea = visibleArea,
    )
  }

  return resolveEditorScrollOffset(
    currentScroll = currentScroll,
    targetTopInContent = targetTopInContent,
    targetBottomInContent = targetBottomInContent,
    range = keepVisibleRange,
  )
}

private fun resolveCenteredVisibleScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
): Float? {
  val visibleTop = visibleArea.visibleViewportTop
  val visibleBottom = visibleArea.visibleViewportBottom
  if (visibleBottom <= visibleTop) return null

  val targetTopInViewport = targetTopInContent - currentScroll
  val targetBottomInViewport = targetBottomInContent - currentScroll
  if (targetTopInViewport >= visibleTop && targetBottomInViewport <= visibleBottom) {
    return null
  }

  val targetCenter = targetTopInContent + (targetBottomInContent - targetTopInContent) / 2f
  val visibleCenter = visibleTop + (visibleBottom - visibleTop) / 2f
  val targetScroll = targetCenter - visibleCenter
  return if (abs(targetScroll - currentScroll) <= 1f) {
    null
  } else {
    targetScroll
  }
}

internal fun resolveTypewriterScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
  position: Float,
): Float? {
  val targetHeight = (targetBottomInContent - targetTopInContent).coerceAtLeast(0f)
  val targetTopInViewport =
    resolveScrollTargetTop(
      visibleArea = visibleArea,
      position = position,
      targetHeight = targetHeight,
    ) ?: return null
  val targetScroll = targetTopInContent - targetTopInViewport
  return if (abs(targetScroll - currentScroll) <= 1f) {
    null
  } else {
    targetScroll
  }
}

private fun resolveKeepVisibleRange(visibleArea: EditorVisibleArea): VerticalSpan {
  val top = visibleArea.visibleViewportTop + CursorVisibleMargin
  val bottom = visibleArea.visibleViewportBottom - CursorVisibleMargin
  return if (bottom <= top) {
    VerticalSpan()
  } else {
    VerticalSpan(top = top, bottom = bottom)
  }
}

private fun resolveKeepVisibleBottomSpacerHeight(
  visibleArea: EditorVisibleArea,
  baseBottomSpace: Float,
): Float {
  return (visibleArea.bottomOcclusion + CursorVisibleMargin - baseBottomSpace).coerceAtLeast(0f)
}

internal fun resolveScrollTargetTop(
  visibleArea: EditorVisibleArea,
  position: Float,
  targetHeight: Float,
): Float? {
  val clampedPosition = position.coerceIn(0f, 1f)
  val clampedTargetHeight = targetHeight.coerceAtLeast(0f)
  val usableViewportHeight =
    (visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop).coerceAtLeast(0f)
  if (usableViewportHeight <= 0f) {
    return null
  }
  val availableRange = (usableViewportHeight - clampedTargetHeight).coerceAtLeast(0f)
  return visibleArea.visibleViewportTop + availableRange * clampedPosition
}

private fun resolveTypewriterBottomSpacerHeight(
  visibleArea: EditorVisibleArea,
  baseBottomSpace: Float,
  distanceToPagesBottom: Float?,
  position: Float,
  targetLineHeight: Float,
): Float {
  val clampedTargetLineHeight = targetLineHeight.coerceAtLeast(0f)
  val usableViewportHeight =
    (visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop).coerceAtLeast(0f)
  val availableRange = (usableViewportHeight - clampedTargetLineHeight).coerceAtLeast(0f)
  val spaceNeededBelowTargetTop =
    visibleArea.bottomOcclusion + (1f - position) * availableRange + clampedTargetLineHeight
  val resolvedDistanceToPagesBottom =
    distanceToPagesBottom?.coerceAtLeast(0f) ?: (baseBottomSpace + clampedTargetLineHeight)
  val requiredPadding = spaceNeededBelowTargetTop - resolvedDistanceToPagesBottom
  return requiredPadding.coerceAtLeast(TypewriterMinBottomPadding)
}
