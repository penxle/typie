package co.typie.editor.scroll

import co.typie.editor.VerticalSpan
import kotlin.math.abs

internal const val CursorVisibleMargin = 60f
private const val TypewriterMinBottomPadding = 48f

internal data class EditorAutoScrollPolicy(
  val typewriterActive: Boolean,
  val typewriterPosition: Float,
  val targetTop: Float?,
  val targetLineHeight: Float,
  val bottomPadding: Float,
  val configuration: EditorAutoScrollPolicyConfiguration,
) {
  val targetBottom: Float?
    get() = targetTop?.plus(targetLineHeight)
}

internal data class EditorAutoScrollPolicyConfiguration(
  val bottomScrollReserveArea: EditorVisibleArea,
  val baseBottomSpace: Float,
  val pageBottomRevealPadding: Float,
  val typewriterEnabled: Boolean,
)

internal fun resolveEditorAutoScrollPolicy(
  visibleArea: EditorVisibleArea,
  bottomScrollReserveArea: EditorVisibleArea = visibleArea,
  baseBottomSpace: Float = 0f,
  pageBottomRevealPadding: Float = 0f,
  typewriterEnabled: Boolean = false,
  typewriterActive: Boolean = typewriterEnabled,
  typewriterPosition: Float = 0.5f,
  targetLineHeight: Float = 0f,
): EditorAutoScrollPolicy {
  val useTypewriter = typewriterEnabled && typewriterActive
  val resolvedTypewriterPosition = typewriterPosition.coerceIn(0f, 1f)
  val resolvedTargetLineHeight = targetLineHeight.coerceAtLeast(0f)
  val targetTop =
    resolveScrollTargetTop(
      visibleArea = visibleArea,
      position = resolvedTypewriterPosition,
      targetHeight = resolvedTargetLineHeight,
    )
  val keepVisibleBottomPadding =
    resolveKeepVisibleBottomPadding(
      visibleArea = bottomScrollReserveArea,
      baseBottomSpace = baseBottomSpace,
    )
  val modeBottomPadding =
    if (useTypewriter) {
      resolveTypewriterBottomPadding(
        visibleArea = bottomScrollReserveArea,
        baseBottomSpace = baseBottomSpace,
        position = resolvedTypewriterPosition,
        targetLineHeight = resolvedTargetLineHeight,
      )
    } else {
      keepVisibleBottomPadding
    }

  return EditorAutoScrollPolicy(
    typewriterActive = useTypewriter,
    typewriterPosition = resolvedTypewriterPosition,
    targetTop = targetTop,
    targetLineHeight = resolvedTargetLineHeight,
    bottomPadding =
      maxOf(keepVisibleBottomPadding, modeBottomPadding, pageBottomRevealPadding.coerceAtLeast(0f)),
    configuration =
      EditorAutoScrollPolicyConfiguration(
        bottomScrollReserveArea = bottomScrollReserveArea,
        baseBottomSpace = baseBottomSpace,
        pageBottomRevealPadding = pageBottomRevealPadding,
        typewriterEnabled = typewriterEnabled,
      ),
  )
}

internal fun EditorAutoScrollPolicy.resolveForState(
  visibleArea: EditorVisibleArea,
  typewriterActive: Boolean,
  targetLineHeight: Float,
): EditorAutoScrollPolicy =
  resolveEditorAutoScrollPolicy(
    visibleArea = visibleArea,
    bottomScrollReserveArea = configuration.bottomScrollReserveArea,
    baseBottomSpace = configuration.baseBottomSpace,
    pageBottomRevealPadding = configuration.pageBottomRevealPadding,
    typewriterEnabled = configuration.typewriterEnabled,
    typewriterActive = typewriterActive,
    typewriterPosition = typewriterPosition,
    targetLineHeight = targetLineHeight,
  )

internal fun resolveEditorScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  range: VerticalSpan,
  maximumScrollY: Float = Float.POSITIVE_INFINITY,
  oversizedMinimumVisibleHeight: Float? = null,
): Float? {
  if (!range.isValid) {
    return null
  }

  val targetTopInViewport = targetTopInContent - currentScroll
  val targetBottomInViewport = targetBottomInContent - currentScroll
  val targetHeight = (targetBottomInContent - targetTopInContent).coerceAtLeast(0f)

  if (targetHeight > range.height) {
    if (oversizedMinimumVisibleHeight != null) {
      val minimumVisibleHeight = oversizedMinimumVisibleHeight.coerceIn(0f, range.height)
      val visibleHeight =
        (minOf(targetBottomInViewport, range.bottom) - maxOf(targetTopInViewport, range.top))
          .coerceAtLeast(0f)
      if (visibleHeight >= minimumVisibleHeight) {
        return null
      }

      val targetScroll =
        when {
          targetTopInViewport > range.top ->
            targetTopInContent - (range.bottom - minimumVisibleHeight)
          targetBottomInViewport < range.bottom ->
            targetBottomInContent - (range.top + minimumVisibleHeight)
          else -> null
        } ?: return null
      return resolveFeasibleScrollOffset(
        targetScroll = targetScroll,
        currentScroll = currentScroll,
        maximumScrollY = maximumScrollY,
      )
    }

    val targetScroll =
      when {
        targetTopInViewport <= range.top && targetBottomInViewport >= range.bottom -> null
        targetBottomInViewport > range.bottom -> targetBottomInContent - range.bottom
        targetTopInViewport < range.top -> targetTopInContent - range.top
        else -> null
      } ?: return null
    return resolveFeasibleScrollOffset(
      targetScroll = targetScroll,
      currentScroll = currentScroll,
      maximumScrollY = maximumScrollY,
    )
  }

  val targetScroll =
    when {
      targetBottomInViewport > range.bottom -> targetBottomInContent - range.bottom
      targetTopInViewport < range.top -> targetTopInContent - range.top
      else -> null
    } ?: return null
  return resolveFeasibleScrollOffset(
    targetScroll = targetScroll,
    currentScroll = currentScroll,
    maximumScrollY = maximumScrollY,
  )
}

internal fun resolveKeepVisibleScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
  maximumScrollY: Float = Float.POSITIVE_INFINITY,
  oversizedMinimumVisibleHeight: Float? = null,
): Float? {
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  if (!keepVisibleRange.isValid) {
    return resolveCenteredVisibleScrollOffset(
      currentScroll = currentScroll,
      targetTopInContent = targetTopInContent,
      targetBottomInContent = targetBottomInContent,
      visibleArea = visibleArea,
      maximumScrollY = maximumScrollY,
    )
  }

  return resolveEditorScrollOffset(
    currentScroll = currentScroll,
    targetTopInContent = targetTopInContent,
    targetBottomInContent = targetBottomInContent,
    range = keepVisibleRange,
    maximumScrollY = maximumScrollY,
    oversizedMinimumVisibleHeight = oversizedMinimumVisibleHeight,
  )
}

internal fun resolveRevealScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
  maximumScrollY: Float = Float.POSITIVE_INFINITY,
): Float? {
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  if (!keepVisibleRange.isValid) {
    return resolveCenteredVisibleScrollOffset(
      currentScroll = currentScroll,
      targetTopInContent = targetTopInContent,
      targetBottomInContent = targetBottomInContent,
      visibleArea = visibleArea,
      maximumScrollY = maximumScrollY,
    )
  }

  val targetHeight = (targetBottomInContent - targetTopInContent).coerceAtLeast(0f)
  if (targetHeight <= keepVisibleRange.height) {
    return resolveEditorScrollOffset(
      currentScroll = currentScroll,
      targetTopInContent = targetTopInContent,
      targetBottomInContent = targetBottomInContent,
      range = keepVisibleRange,
      maximumScrollY = maximumScrollY,
    )
  }

  return resolveFeasibleScrollOffset(
    targetScroll = targetTopInContent - keepVisibleRange.top,
    currentScroll = currentScroll,
    maximumScrollY = maximumScrollY,
  )
}

private fun resolveCenteredVisibleScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
  maximumScrollY: Float,
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
  return resolveFeasibleScrollOffset(
    targetScroll = targetScroll,
    currentScroll = currentScroll,
    maximumScrollY = maximumScrollY,
  )
}

internal fun resolveTypewriterScrollOffset(
  currentScroll: Float,
  targetTopInContent: Float,
  targetBottomInContent: Float,
  visibleArea: EditorVisibleArea,
  position: Float,
  maximumScrollY: Float = Float.POSITIVE_INFINITY,
): Float? {
  val targetHeight = (targetBottomInContent - targetTopInContent).coerceAtLeast(0f)
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  if (!keepVisibleRange.isValid || targetHeight > keepVisibleRange.height) {
    return resolveKeepVisibleScrollOffset(
      currentScroll = currentScroll,
      targetTopInContent = targetTopInContent,
      targetBottomInContent = targetBottomInContent,
      visibleArea = visibleArea,
      maximumScrollY = maximumScrollY,
    )
  }
  val targetTopInViewport =
    resolveScrollTargetTop(
      visibleArea = visibleArea,
      position = position,
      targetHeight = targetHeight,
    ) ?: return null
  val targetScroll = targetTopInContent - targetTopInViewport
  return resolveFeasibleScrollOffset(
    targetScroll = targetScroll,
    currentScroll = currentScroll,
    maximumScrollY = maximumScrollY,
  )
}

private fun resolveFeasibleScrollOffset(
  targetScroll: Float,
  currentScroll: Float,
  maximumScrollY: Float,
): Float? {
  val feasibleScroll = targetScroll.coerceIn(0f, maximumScrollY)
  return feasibleScroll.takeUnless { abs(it - currentScroll) <= 1f }
}

internal fun resolveKeepVisibleRange(visibleArea: EditorVisibleArea): VerticalSpan {
  val top = visibleArea.visibleViewportTop + CursorVisibleMargin
  val bottom = visibleArea.visibleViewportBottom - CursorVisibleMargin
  return if (bottom <= top) {
    VerticalSpan()
  } else {
    VerticalSpan(top = top, bottom = bottom)
  }
}

private fun resolveKeepVisibleBottomPadding(
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

private fun resolveTypewriterBottomPadding(
  visibleArea: EditorVisibleArea,
  baseBottomSpace: Float,
  position: Float,
  targetLineHeight: Float,
): Float {
  val clampedTargetLineHeight = targetLineHeight.coerceAtLeast(0f)
  val usableViewportHeight =
    (visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop).coerceAtLeast(0f)
  val availableRange = (usableViewportHeight - clampedTargetLineHeight).coerceAtLeast(0f)
  val spaceNeededBelowTargetTop =
    visibleArea.bottomOcclusion + (1f - position) * availableRange + clampedTargetLineHeight
  val intrinsicSpaceBelowTargetTop = baseBottomSpace + clampedTargetLineHeight
  val requiredPadding = spaceNeededBelowTargetTop - intrinsicSpaceBelowTargetTop
  return requiredPadding.coerceAtLeast(TypewriterMinBottomPadding)
}
