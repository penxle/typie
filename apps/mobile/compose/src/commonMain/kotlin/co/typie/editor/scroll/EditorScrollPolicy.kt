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
  val bottomPadding: Float,
) {
  val typewriterTargetBottom: Float?
    get() = typewriterTargetTop?.plus(typewriterCursorHeight)
}

internal fun resolveEditorScrollPolicy(
  visibleArea: EditorVisibleArea,
  intrinsicBottomSpace: Float = 0f,
  typewriterEnabled: Boolean = false,
  typewriterPosition: Float = 0.5f,
  cursorHeight: Float = 0f,
): EditorScrollPolicy {
  val resolvedTypewriterPosition = typewriterPosition.coerceIn(0f, 1f)
  val keepVisibleRange = resolveKeepVisibleRange(visibleArea)
  val resolvedCursorHeight = max(0f, cursorHeight)
  val typewriterTargetTop =
    resolveTypewriterTargetTop(
      visibleArea = visibleArea,
      position = resolvedTypewriterPosition,
      cursorHeight = resolvedCursorHeight,
    )
  val keepVisibleBottomPadding =
    resolveKeepVisibleBottomPadding(
      visibleArea = visibleArea,
      intrinsicBottomSpace = intrinsicBottomSpace,
    )

  return EditorScrollPolicy(
    mode =
      if (typewriterEnabled) EditorScrollMode.Typewriter else EditorScrollMode.KeepCursorVisible,
    typewriterPosition = resolvedTypewriterPosition,
    keepVisibleRange = keepVisibleRange,
    typewriterTargetTop = typewriterTargetTop,
    typewriterCursorHeight = resolvedCursorHeight,
    bottomPadding =
      if (typewriterEnabled) {
        resolveTypewriterBottomPadding(
          visibleArea = visibleArea,
          intrinsicBottomSpace = intrinsicBottomSpace,
          position = resolvedTypewriterPosition,
          cursorHeight = resolvedCursorHeight,
        )
      } else {
        keepVisibleBottomPadding
      },
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
  // 작아서 guard 기준선 근처에서 몇 dp의 여유 스크롤이 남을 수 있다.
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
  val cursorHeight = max(0f, cursorBottomInContent - cursorTopInContent)
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

private fun resolveKeepVisibleBottomPadding(
  visibleArea: EditorVisibleArea,
  intrinsicBottomSpace: Float,
): Float {
  return max(0f, visibleArea.bottomOcclusion + CursorVisibleMargin - intrinsicBottomSpace)
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
  val clampedCursorHeight = max(0f, cursorHeight)
  val usableViewportHeight =
    max(0f, visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop)
  if (usableViewportHeight <= 0f) {
    return null
  }
  val availableRange = max(0f, usableViewportHeight - clampedCursorHeight)
  return visibleArea.visibleViewportTop + availableRange * clampedPosition
}

private fun resolveTypewriterBottomPadding(
  visibleArea: EditorVisibleArea,
  intrinsicBottomSpace: Float,
  position: Float,
  cursorHeight: Float,
): Float {
  // TODO(editor-parity): collapsed selection에서는 실제 selection head 표시 높이보다 작은
  // cursor 높이만 써서 typewriter bottom padding이 부족하게 계산되고, 그 결과 문서 끝에서
  // 몇 dp의 추가 스크롤이 남는다. 같은 높이 차이 때문에 일반 keep-visible(cursor guard)도
  // guard 기준선에서 약간의 여유 스크롤이 남는다. 웹/플러터처럼 selection-head leading과
  // presented height를 반영하도록 맞춰야 한다.
  val clampedCursorHeight = max(0f, cursorHeight)
  val usableViewportHeight =
    max(0f, visibleArea.visibleViewportBottom - visibleArea.visibleViewportTop)
  val availableRange = max(0f, usableViewportHeight - clampedCursorHeight)
  val spaceNeededBelowCursorTop =
    visibleArea.bottomOcclusion + (1f - position) * availableRange + clampedCursorHeight
  val intrinsicSpaceBelowLastLine = intrinsicBottomSpace + clampedCursorHeight
  val requiredPadding = spaceNeededBelowCursorTop - intrinsicSpaceBelowLastLine
  return max(TypewriterMinBottomPadding, max(requiredPadding, 0f))
}
