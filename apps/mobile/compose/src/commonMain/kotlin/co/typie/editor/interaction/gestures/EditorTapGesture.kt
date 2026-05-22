package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorState
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionCommand
import co.typie.editor.interaction.semantics.dispatchPrimaryClick
import co.typie.editor.interaction.semantics.hasRangeSelection
import co.typie.editor.interaction.semantics.isSelectionHit

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

internal fun EditorTapGesture.handlePointerDown(
  pointerId: Long,
  position: Offset,
  nowMillis: Long,
  tapEnabled: Boolean,
  doubleTapDrag: EditorDoubleTapDragGesture,
  context: EditorGestureContext,
): Boolean {
  if (
    !tapEnabled || !context.can(EditorInteractionCommand.TapDown) || isIgnoringUntilAllPointersUp
  ) {
    return false
  }

  startActivePointer(pointerId = pointerId, position = position)
  if (nextTapCount(position = position, nowMillis = nowMillis) == 2) {
    markTapDispatched()
    context.cancelTapDispatch()
    dispatchTap(
      position = position,
      nowMillis = nowMillis,
      clickCount = 2,
      doubleTapDrag = doubleTapDrag,
      context = context,
    ) {
      doubleTapDrag.prepareForDrag(position = position, tap = this, context = context)
    }
    return true
  }

  markTapPending()
  context.scheduleTapDispatch(dispatchAtMillis = nowMillis + EditorTapDispatchDelayMillis)
  return false
}

internal fun EditorTapGesture.trackPointerMove(
  pointerId: Long,
  position: Offset,
  context: EditorGestureContext,
) {
  if (onPointerMove(pointerId = pointerId, position = position)) {
    context.cancelTapDispatch()
  }
}

internal fun EditorTapGesture.handlePointerUp(
  pointerId: Long,
  position: Offset,
  nowMillis: Long,
  doubleTapDrag: EditorDoubleTapDragGesture,
  context: EditorGestureContext,
): Boolean {
  val canFinishTap = context.can(EditorInteractionCommand.TapUp)
  val shouldConsumeTap = shouldConsumePointerUp(pointerId = pointerId, canFinish = canFinishTap)
  val clickCount =
    onPointerUp(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      canFinish = canFinishTap,
    )
  if (!canFinishTap) {
    context.cancelTapDispatch()
  }
  clickCount?.let {
    dispatchTap(
      position = position,
      nowMillis = nowMillis,
      clickCount = it,
      doubleTapDrag = doubleTapDrag,
      context = context,
      beforeLaunch = {},
    )
  }
  return shouldConsumeTap
}

internal fun EditorTapGesture.handleTapTimer(
  nowMillis: Long,
  doubleTapDrag: EditorDoubleTapDragGesture,
  context: EditorGestureContext,
) {
  val position = activePosition ?: return
  if (!canDispatchTapTimer) {
    return
  }
  val clickCount = nextTapCount(position = position, nowMillis = nowMillis)
  if (clickCount == 1 && isSelectionHit(position = position, context = context)) {
    markTapDispatched()
    return
  }
  if (clickCount == 1 && context.semantics.cursorMove.hasRangeSelection(context.editor)) {
    return
  }
  markTapDispatched()
  dispatchTap(
    position = position,
    nowMillis = nowMillis,
    clickCount = clickCount,
    doubleTapDrag = doubleTapDrag,
    context = context,
    beforeLaunch = {},
  )
}

private fun EditorTapGesture.dispatchTap(
  position: Offset,
  nowMillis: Long,
  clickCount: Int,
  doubleTapDrag: EditorDoubleTapDragGesture,
  context: EditorGestureContext,
  beforeLaunch: () -> Unit,
): Boolean {
  val point = context.resolvePoint(positionInNode = position) ?: return false
  if (!context.can(EditorInteractionCommand.TapDispatch(page = point.page))) {
    return false
  }
  recordTap(nowMillis = nowMillis, position = position, clickCount = clickCount)
  val editor = context.editor
  if (clickCount == 1 && context.semantics.cursorMove.isSelectionHit(editor, point)) {
    return false
  }
  val previousCursor = editor.cursor
  context.requestFocus(editor)
  beforeLaunch()
  val tap = this
  context.launchInteraction {
    val dispatched =
      context.semantics.cursorMove.dispatchPrimaryClick(
        editor = editor,
        point = point,
        clickCount = clickCount,
        beforeCommit = { snapshot ->
          if (clickCount == 1 && shouldRequestSingleTapBringIntoView(previousCursor, snapshot)) {
            context.requestCurrentCursorLine(version = snapshot.version)
          }
        },
      )
    if (dispatched && clickCount == 2) {
      doubleTapDrag.onWordSelectionCommitted(tap = tap, context = context)
    }
  }
  return true
}

private fun EditorTapGesture.isSelectionHit(
  position: Offset,
  context: EditorGestureContext,
): Boolean {
  val point = context.resolvePoint(positionInNode = position) ?: return true
  return point.page < 0 || context.semantics.cursorMove.isSelectionHit(context.editor, point)
}

private fun shouldRequestSingleTapBringIntoView(
  previousCursor: CursorMetrics?,
  nextState: EditorState,
): Boolean {
  val nextCursor = nextState.cursor ?: return false
  if (
    nextState.selection.isCollapsed() &&
      previousCursor != null &&
      nextCursor.isSamePosition(previousCursor)
  ) {
    // TODO(editor-parity): same-cursor single tap should open the context menu slot.
    return false
  }
  return true
}

private fun Selection?.isCollapsed(): Boolean = this == null || anchor == head

private fun CursorMetrics.isSamePosition(other: CursorMetrics): Boolean =
  pageIdx == other.pageIdx &&
    caret.x == other.caret.x &&
    caret.y == other.caret.y &&
    line.y == other.line.y
