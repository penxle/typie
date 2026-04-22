package co.typie.editor.scroll

import co.typie.editor.VerticalSpan
import kotlin.math.abs
import kotlin.math.max

internal const val CursorVisibleMargin = 60f
private const val TypewriterMinBottomPadding = 48f

internal enum class EditorScrollMode {
  KeepCursorVisible,
  Typewriter,
}

internal data class EditorScrollPolicy(
  val mode: EditorScrollMode,
  val typewriterPosition: Float,
  val keepVisibleRange: VerticalSpan,
  val typewriterTargetTop: Float?,
  val typewriterCursorHeight: Float,
  val bottomSpacerHeight: Float,
) {
  val typewriterTargetBottom: Float?
    get() = typewriterTargetTop?.plus(typewriterCursorHeight)
}

internal fun resolveEditorScrollPolicy(
  visibleArea: EditorVisibleArea,
  baseBottomSpace: Float = 0f,
  distanceToPagesBottom: Float? = null,
  pageBottomRevealSpacerHeight: Float = 0f,
  typewriterEnabled: Boolean = false,
  typewriterPosition: Float = 0.5f,
  cursorHeight: Float = 0f,
): EditorScrollPolicy {
  val resolvedTypewriterPosition = typewriterPosition.coerceIn(0f, 1f)
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  val resolvedCursorHeight = cursorHeight.coerceAtLeast(0f)
  val typewriterTargetTop =
    resolveTypewriterTargetTop(
      visibleArea = visibleArea,
      position = resolvedTypewriterPosition,
      cursorHeight = resolvedCursorHeight,
    )
  val keepVisibleBottomSpacerHeight =
    resolveKeepVisibleBottomSpacerHeight(
      visibleArea = visibleArea,
      baseBottomSpace = baseBottomSpace,
    )
  val cursorPolicyBottomSpacerHeight =
    if (typewriterEnabled) {
      resolveTypewriterBottomSpacerHeight(
        visibleArea = visibleArea,
        baseBottomSpace = baseBottomSpace,
        distanceToPagesBottom = distanceToPagesBottom,
        position = resolvedTypewriterPosition,
        cursorHeight = resolvedCursorHeight,
      )
    } else {
      keepVisibleBottomSpacerHeight
    }

  return EditorScrollPolicy(
    mode =
      if (typewriterEnabled) EditorScrollMode.Typewriter else EditorScrollMode.KeepCursorVisible,
    typewriterPosition = resolvedTypewriterPosition,
    keepVisibleRange = keepVisibleRange,
    typewriterTargetTop = typewriterTargetTop,
    typewriterCursorHeight = resolvedCursorHeight,
    bottomSpacerHeight =
      max(cursorPolicyBottomSpacerHeight, pageBottomRevealSpacerHeight.coerceAtLeast(0f)),
  )
}

internal fun resolveEditorScrollTarget(
  currentScroll: Float,
  cursorTopInContent: Float,
  cursorBottomInContent: Float,
  range: VerticalSpan,
): Float? {
  // TODO(editor-parity): keep-visible(cursor guard)도 현재 cursor rect의 top/bottom만 기준으로
  // 계산하고 있다. collapsed selection에서는 이 rect 높이가 실제 selection head 표시 높이보다
  // 작아서 guard 기준선 근처에서 여유 스크롤이 남고, 부족분은 displayZoom이 커질수록 같이
  // 커진다.
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

internal fun resolveTypewriterScrollTarget(
  currentScroll: Float,
  cursorTopInContent: Float,
  cursorBottomInContent: Float,
  visibleArea: EditorVisibleArea,
  position: Float,
): Float? {
  val cursorHeight = (cursorBottomInContent - cursorTopInContent).coerceAtLeast(0f)
  val targetTopInViewport =
    resolveTypewriterTargetTop(
      visibleArea = visibleArea,
      position = position,
      cursorHeight = cursorHeight,
    ) ?: return null
  val targetScroll = cursorTopInContent - targetTopInViewport
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

internal fun resolveTypewriterTargetTop(
  visibleArea: EditorVisibleArea,
  position: Float,
  cursorHeight: Float,
): Float? {
  // TODO(editor-parity): selection head / composition 표시 bounds를 쓰도록 바꿔야 한다.
  // 지금은 cursor 높이만 써서 collapsed selection에서는 실제 표시 높이보다 작은 값으로
  // typewriter 기준을 잡고 있고, non-collapsed selection도 head 기준이 아니라 웹/플러터와
  // 패리티가 맞지 않는다.
  val clampedPosition = position.coerceIn(0f, 1f)
  val clampedCursorHeight = cursorHeight.coerceAtLeast(0f)
  val usableViewportHeight =
    (visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop).coerceAtLeast(0f)
  if (usableViewportHeight <= 0f) {
    return null
  }
  val availableRange = (usableViewportHeight - clampedCursorHeight).coerceAtLeast(0f)
  return visibleArea.visibleViewportTop + availableRange * clampedPosition
}

private fun resolveTypewriterBottomSpacerHeight(
  visibleArea: EditorVisibleArea,
  baseBottomSpace: Float,
  distanceToPagesBottom: Float?,
  position: Float,
  cursorHeight: Float,
): Float {
  // TODO(editor-parity): collapsed selection에서는 실제 selection head 표시 높이보다 작은
  // cursor 높이만 써서 typewriter bottom padding이 부족하게 계산되고, 그 결과 문서 끝에서
  // 추가 스크롤이 남는다. 이 부족분도 displayZoom이 커질수록 같이 커진다. 같은 높이 차이
  // 때문에 일반 keep-visible(cursor guard)도 guard 기준선에서 여유 스크롤이 남는다.
  // 웹/플러터처럼 selection-head leading과 presented height를 반영하도록 맞춰야 한다.
  val clampedCursorHeight = cursorHeight.coerceAtLeast(0f)
  val usableViewportHeight =
    (visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop).coerceAtLeast(0f)
  val availableRange = (usableViewportHeight - clampedCursorHeight).coerceAtLeast(0f)
  val spaceNeededBelowCursorTop =
    visibleArea.bottomOcclusion + (1f - position) * availableRange + clampedCursorHeight
  val resolvedDistanceToPagesBottom =
    distanceToPagesBottom?.coerceAtLeast(0f) ?: (baseBottomSpace + clampedCursorHeight)
  val requiredPadding = spaceNeededBelowCursorTop - resolvedDistanceToPagesBottom
  return requiredPadding.coerceAtLeast(TypewriterMinBottomPadding)
}
