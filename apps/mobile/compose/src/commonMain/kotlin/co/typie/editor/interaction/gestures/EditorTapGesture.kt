package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset

private const val EditorTapDownDelayMillis = 100L
private const val EditorTapTimerDelayMillis = 150L
internal const val EditorTapDispatchDelayMillis =
  EditorTapDownDelayMillis + EditorTapTimerDelayMillis

private const val ConsecutiveTapMaxIntervalMillis = 300L
private const val ConsecutiveTapMaxDistancePx = 20f

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

  val pressedPointerCount: Int
    get() = pressedPointerIds.size

  val isIgnoringUntilAllPointersUp: Boolean
    get() = ignoringUntilAllPointersUp

  val hasActivePointer: Boolean
    get() = activePointerId != null

  val activePosition: Offset?
    get() = if (activePointerId == null) null else downPosition

  val canDispatchTapTimer: Boolean
    get() =
      activePointerId != null && !movedPastTapSlop && !tapDispatched && !ignoringUntilAllPointersUp

  fun updateTapSlop(tapSlopPx: Float) {
    this.tapSlopPx = tapSlopPx
  }

  fun addPressedPointer(pointerId: Long) {
    pressedPointerIds += pointerId
  }

  fun startActivePointer(pointerId: Long, position: Offset) {
    activePointerId = pointerId
    downPosition = position
    movedPastTapSlop = false
    tapDispatched = false
  }

  fun cancelActivePointerAndIgnoreUntilAllPointersUp() {
    clearActivePointer()
    ignoringUntilAllPointersUp = true
  }

  fun onPointerMove(pointerId: Long, position: Offset): Boolean {
    if (ignoringUntilAllPointersUp || activePointerId != pointerId) {
      return false
    }
    if ((position - downPosition).getDistance() > tapSlopPx) {
      movedPastTapSlop = true
      return true
    }
    return false
  }

  fun markTapDispatched() {
    tapDispatched = true
  }

  fun markTapPending() {
    tapDispatched = false
  }

  fun shouldConsumePointerUp(pointerId: Long, canFinish: Boolean): Boolean =
    canFinish && !ignoringUntilAllPointersUp && activePointerId == pointerId && !movedPastTapSlop

  fun onPointerUp(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    canFinish: Boolean = true,
  ): Int? {
    pressedPointerIds -= pointerId
    if (ignoringUntilAllPointersUp) {
      if (pressedPointerIds.isEmpty()) {
        ignoringUntilAllPointersUp = false
      }
      return null
    }

    if (activePointerId != pointerId) {
      return null
    }
    if (!canFinish) {
      clearActivePointer()
      return null
    }

    val clickCount =
      if (!movedPastTapSlop && !tapDispatched) {
        nextTapCount(position = position, nowMillis = nowMillis)
      } else {
        null
      }
    clearActivePointer()
    return clickCount
  }

  fun cancelActivePointerStream(): Boolean {
    val hadActivePointer = activePointerId != null
    clearActivePointer()
    pressedPointerIds.clear()
    ignoringUntilAllPointersUp = false
    return hadActivePointer
  }

  fun reset() {
    clearActivePointer()
    pressedPointerIds.clear()
    ignoringUntilAllPointersUp = false
    lastTapTimeMillis = null
    lastTapPosition = null
  }

  private fun clearActivePointer() {
    activePointerId = null
    downPosition = Offset.Zero
    movedPastTapSlop = false
    tapDispatched = false
  }

  fun recordTap(nowMillis: Long, position: Offset, clickCount: Int) {
    if (clickCount == 2) {
      lastTapTimeMillis = null
      lastTapPosition = null
    } else {
      lastTapTimeMillis = nowMillis
      lastTapPosition = position
    }
  }

  fun nextTapCount(position: Offset, nowMillis: Long): Int =
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
