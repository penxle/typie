package co.typie.editor.gesture

import androidx.compose.ui.geometry.Offset

private const val EditorTapDownDelayMillis = 100L
private const val EditorTapTimerDelayMillis = 150L
internal const val EditorTapDispatchDelayMillis =
  EditorTapDownDelayMillis + EditorTapTimerDelayMillis

private const val ConsecutiveTapMaxIntervalMillis = 300L
private const val ConsecutiveTapMaxDistancePx = 20f

internal data class EditorInteractionTapDispatch(val position: Offset, val clickCount: Int)

internal data class EditorInteractionPointerResult(
  val consume: Boolean = false,
  val tapDispatch: EditorInteractionTapDispatch? = null,
  val cancelTapDispatch: Boolean = false,
  val cancelPointerStream: Boolean = false,
)

internal class EditorTapGesture(
  private var tapSlopPx: Float,
  private val consecutiveTapMaxIntervalMillis: Long = ConsecutiveTapMaxIntervalMillis,
  private val consecutiveTapMaxDistancePx: Float = ConsecutiveTapMaxDistancePx,
) {
  private val pressedPointerIds = mutableSetOf<Long>()
  private var activePointerId: Long? = null
  private var downPosition = Offset.Zero
  private var movedPastTapSlop = false
  private var tapDispatched = false
  private var ignoringUntilAllPointersUp = false
  private var lastTapTimeMillis: Long? = null
  private var lastTapPosition: Offset? = null
  private var lastTapCount = 0

  val pressedPointerCount: Int
    get() = pressedPointerIds.size

  val isIgnoringUntilAllPointersUp: Boolean
    get() = ignoringUntilAllPointersUp

  val hasActivePointer: Boolean
    get() = activePointerId != null

  fun updateTapSlop(tapSlopPx: Float) {
    this.tapSlopPx = tapSlopPx
  }

  fun onPointerDown(
    pointerId: Long,
    position: Offset,
    canStart: () -> Boolean,
  ): EditorInteractionPointerResult {
    pressedPointerIds += pointerId
    if (!canStart()) {
      return EditorInteractionPointerResult()
    }
    if (ignoringUntilAllPointersUp) {
      return EditorInteractionPointerResult()
    }

    if (activePointerId != null) {
      clearActivePointer()
      ignoringUntilAllPointersUp = true
      return EditorInteractionPointerResult(cancelTapDispatch = true, cancelPointerStream = true)
    }

    activePointerId = pointerId
    downPosition = position
    movedPastTapSlop = false
    tapDispatched = false
    return EditorInteractionPointerResult()
  }

  fun onPointerMove(pointerId: Long, position: Offset): EditorInteractionPointerResult {
    if (ignoringUntilAllPointersUp || activePointerId != pointerId) {
      return EditorInteractionPointerResult()
    }
    if ((position - downPosition).getDistance() > tapSlopPx) {
      movedPastTapSlop = true
      return EditorInteractionPointerResult(cancelTapDispatch = true)
    }
    return EditorInteractionPointerResult()
  }

  fun onTapTimer(
    nowMillis: Long,
    isSelectionHit: (Offset) -> Boolean = { false },
    hasRangeSelection: () -> Boolean = { false },
  ): EditorInteractionTapDispatch? {
    if (
      activePointerId == null || movedPastTapSlop || tapDispatched || ignoringUntilAllPointersUp
    ) {
      return null
    }
    val clickCount = nextTapCount(position = downPosition, nowMillis = nowMillis)
    if (clickCount == 1 && isSelectionHit(downPosition)) {
      tapDispatched = true
      return null
    }
    if (clickCount == 1 && hasRangeSelection()) {
      return null
    }
    tapDispatched = true
    return buildTapDispatch(nowMillis = nowMillis, position = downPosition, clickCount = clickCount)
  }

  fun onPointerUp(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    canFinish: () -> Boolean,
  ): EditorInteractionPointerResult {
    pressedPointerIds -= pointerId
    if (ignoringUntilAllPointersUp) {
      if (pressedPointerIds.isEmpty()) {
        ignoringUntilAllPointersUp = false
      }
      return EditorInteractionPointerResult()
    }

    if (activePointerId != pointerId) {
      return EditorInteractionPointerResult()
    }
    if (!canFinish()) {
      clearActivePointer()
      return EditorInteractionPointerResult(cancelTapDispatch = true)
    }

    val shouldConsume = !movedPastTapSlop
    val dispatch =
      if (!movedPastTapSlop && !tapDispatched) {
        buildTapDispatch(nowMillis = nowMillis, position = position)
      } else {
        null
      }
    clearActivePointer()
    return EditorInteractionPointerResult(consume = shouldConsume, tapDispatch = dispatch)
  }

  fun cancelActivePointerStream(): EditorInteractionPointerResult {
    val hadActivePointer = activePointerId != null
    clearActivePointer()
    pressedPointerIds.clear()
    ignoringUntilAllPointersUp = false
    return EditorInteractionPointerResult(
      cancelTapDispatch = true,
      cancelPointerStream = hadActivePointer,
    )
  }

  fun reset() {
    clearActivePointer()
    pressedPointerIds.clear()
    ignoringUntilAllPointersUp = false
    lastTapTimeMillis = null
    lastTapPosition = null
    lastTapCount = 0
  }

  private fun clearActivePointer() {
    activePointerId = null
    downPosition = Offset.Zero
    movedPastTapSlop = false
    tapDispatched = false
  }

  private fun buildTapDispatch(
    nowMillis: Long,
    position: Offset = downPosition,
  ): EditorInteractionTapDispatch {
    val clickCount = nextTapCount(position = position, nowMillis = nowMillis)
    return buildTapDispatch(nowMillis = nowMillis, position = position, clickCount = clickCount)
  }

  private fun buildTapDispatch(
    nowMillis: Long,
    position: Offset,
    clickCount: Int,
  ): EditorInteractionTapDispatch {
    if (clickCount == 2) {
      lastTapTimeMillis = null
      lastTapPosition = null
      lastTapCount = 0
    } else {
      lastTapTimeMillis = nowMillis
      lastTapPosition = position
      lastTapCount = clickCount
    }
    return EditorInteractionTapDispatch(position = position, clickCount = clickCount)
  }

  private fun nextTapCount(position: Offset, nowMillis: Long): Int =
    if (isConsecutiveTap(position = position, nowMillis = nowMillis)) {
      2
    } else {
      1
    }

  private fun isConsecutiveTap(position: Offset, nowMillis: Long): Boolean {
    val previousTime = lastTapTimeMillis ?: return false
    val previousPosition = lastTapPosition ?: return false
    return nowMillis - previousTime < consecutiveTapMaxIntervalMillis &&
      (position - previousPosition).getDistance() < consecutiveTapMaxDistancePx
  }
}
