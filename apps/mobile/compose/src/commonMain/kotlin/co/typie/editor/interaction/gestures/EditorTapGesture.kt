package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.ViewOp
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.isViewportZooming
import co.typie.editor.interaction.sessions.EditorDoubleTapDragSession
import co.typie.platform.Platform

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
  private var activeInputModifiers = InputModifiers()
  private var lastTapTimeMillis: Long? = null
  private var lastTapPosition: Offset? = null
  private var contextMenuVisibleAtPointerDown = false

  val pressedPointerCount: Int
    get() = pressedPointerIds.size

  val isIgnoringUntilAllPointersUp: Boolean
    get() = ignoringUntilAllPointersUp

  val hasActivePointer: Boolean
    get() = activePointerId != null

  val activePosition: Offset?
    get() = if (activePointerId == null) null else downPosition

  val inputModifiersForActivePointer: InputModifiers
    get() = activeInputModifiers

  val canDispatchTapTimer: Boolean
    get() =
      activePointerId != null && !movedPastTapSlop && !tapDispatched && !ignoringUntilAllPointersUp

  fun updateTapSlop(tapSlopPx: Float) {
    this.tapSlopPx = tapSlopPx
  }

  fun addPressedPointer(pointerId: Long) {
    pressedPointerIds += pointerId
  }

  fun startActivePointer(
    pointerId: Long,
    position: Offset,
    inputModifiers: InputModifiers = InputModifiers(),
  ) {
    activePointerId = pointerId
    downPosition = position
    activeInputModifiers = inputModifiers
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
    contextMenuVisibleAtPointerDown = false
  }

  private fun clearActivePointer() {
    activePointerId = null
    downPosition = Offset.Zero
    activeInputModifiers = InputModifiers()
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

  fun clearTapHistory() {
    lastTapTimeMillis = null
    lastTapPosition = null
  }

  fun captureContextMenuStateAtPointerDown(visible: Boolean) {
    contextMenuVisibleAtPointerDown = visible
  }

  fun shouldOpenContextMenuForCurrentTap(): Boolean = !contextMenuVisibleAtPointerDown

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
  inputModifiers: InputModifiers,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
): Boolean {
  if (!tapEnabled || context.mode.isViewportZooming || isIgnoringUntilAllPointersUp) {
    return false
  }

  startActivePointer(pointerId = pointerId, position = position, inputModifiers = inputModifiers)
  captureContextMenuStateAtPointerDown(context.uiState.contextMenu.visible)
  context.uiState.contextMenu.hide()
  if (nextTapCount(position = position, nowMillis = nowMillis) == 2) {
    markTapDispatched()
    context.effects.cancelTapDispatch()
    dispatchTap(
      position = position,
      nowMillis = nowMillis,
      clickCount = 2,
      inputModifiers = inputModifiers,
      doubleTapDrag = doubleTapDrag,
      context = context,
    ) {
      doubleTapDrag.prepareForDrag(position = position, tap = this, context = context)
    }
    return true
  }

  markTapPending()
  context.effects.scheduleTapDispatch(dispatchAtMillis = nowMillis + EditorTapDispatchDelayMillis)
  return false
}

internal fun EditorTapGesture.trackPointerMove(
  pointerId: Long,
  position: Offset,
  context: EditorGestureContext,
): Boolean {
  if (onPointerMove(pointerId = pointerId, position = position)) {
    context.effects.cancelTapDispatch()
    return true
  }
  return false
}

internal fun EditorTapGesture.handlePointerUp(
  pointerId: Long,
  position: Offset,
  nowMillis: Long,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
): Boolean {
  val canFinishTap = !context.mode.isViewportZooming && !doubleTapDrag.dragging
  val shouldConsumeTap = shouldConsumePointerUp(pointerId = pointerId, canFinish = canFinishTap)
  val inputModifiers = inputModifiersForActivePointer
  val clickCount =
    onPointerUp(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      canFinish = canFinishTap,
    )
  if (!canFinishTap) {
    context.effects.cancelTapDispatch()
  }
  clickCount?.let {
    dispatchTap(
      position = position,
      nowMillis = nowMillis,
      clickCount = it,
      inputModifiers = inputModifiers,
      doubleTapDrag = doubleTapDrag,
      context = context,
      beforeLaunch = {},
    )
  }
  return shouldConsumeTap
}

internal fun EditorTapGesture.handleTapTimer(
  nowMillis: Long,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
) {
  val position = activePosition ?: return
  val inputModifiers = inputModifiersForActivePointer
  if (!canDispatchTapTimer) {
    return
  }
  val clickCount = nextTapCount(position = position, nowMillis = nowMillis)
  val point = context.geometry.resolvePoint(positionInNode = position)
  val hitSelection =
    point != null &&
      point.page >= 0 &&
      context.editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (clickCount == 1 && context.platform != Platform.Android && hitSelection) {
    markTapDispatched()
    if (shouldOpenContextMenuForCurrentTap()) {
      context.uiState.contextMenu.show(context.editor.state)
    }
    return
  }
  if (clickCount == 1 && !context.editor.selection.isCollapsed()) {
    return
  }
  markTapDispatched()
  dispatchTap(
    position = position,
    nowMillis = nowMillis,
    clickCount = clickCount,
    inputModifiers = inputModifiers,
    doubleTapDrag = doubleTapDrag,
    context = context,
    beforeLaunch = {},
  )
}

private fun EditorTapGesture.dispatchTap(
  position: Offset,
  nowMillis: Long,
  clickCount: Int,
  inputModifiers: InputModifiers,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
  beforeLaunch: () -> Unit,
): Boolean {
  val point = context.geometry.resolvePoint(positionInNode = position) ?: return false
  if (context.mode.isViewportZooming || point.page < 0) {
    return false
  }
  val editor = context.editor
  if (
    tryHandleInteractiveHit(
      context = context,
      pointPage = point.page,
      pointX = point.x,
      pointY = point.y,
    )
  ) {
    return true
  }
  recordTap(nowMillis = nowMillis, position = position, clickCount = clickCount)
  val wasFocused = context.uiState.focused
  context.semantics.cursorMove.requestFocus(editor)
  if (wasFocused) {
    context.semantics.cursorMove.requestSoftwareKeyboard()
  }
  val hitExistingSelectionAtTap =
    editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (clickCount == 1 && context.platform != Platform.Android && hitExistingSelectionAtTap) {
    if (shouldOpenContextMenuForCurrentTap()) {
      context.uiState.contextMenu.show(editor.state)
    }
    return false
  }
  val previousCursor = editor.cursor
  beforeLaunch()
  val tap = this
  context.semantics.cursorMove.launchPrimaryClick(
    editor = editor,
    point = point,
    clickCount = clickCount,
    inputModifiers = inputModifiers,
    beforeCommit = { snapshot ->
      if (clickCount == 1) {
        when {
          !snapshot.selection.isCollapsed() -> {
            if (!hitExistingSelectionAtTap) {
              context.uiState.contextMenu.show(snapshot)
              context.semantics.cursorMove.requestCurrentSelectionHead(version = snapshot.version)
            } else {
              context.uiState.contextMenu.hide()
            }
          }
          isSameCursorTap(previousCursor, snapshot) -> {
            if (wasFocused && shouldOpenContextMenuForCurrentTap()) {
              context.uiState.contextMenu.show(snapshot)
            }
          }
          else -> {
            context.uiState.contextMenu.hide()
            if (snapshot.cursor != null) {
              context.semantics.cursorMove.requestCurrentSelectionHead(version = snapshot.version)
            }
          }
        }
      }
    },
    afterDispatch = { dispatched ->
      if (dispatched && clickCount == 2) {
        doubleTapDrag.onWordSelectionCommitted(tap = tap, context = context)
      }
    },
  )
  return true
}

private fun isSameCursorTap(previousCursor: CursorMetrics?, nextState: EditorState): Boolean {
  val nextCursor = nextState.cursor ?: return false
  if (
    nextState.selection.isCollapsed() &&
      previousCursor != null &&
      nextCursor.isSamePosition(previousCursor)
  ) {
    return true
  }
  return false
}

private fun CursorMetrics.isSamePosition(other: CursorMetrics): Boolean =
  pageIdx == other.pageIdx &&
    caret.x == other.caret.x &&
    caret.y == other.caret.y &&
    line.y == other.line.y

private fun tryHandleInteractiveHit(
  context: EditorGestureContext,
  pointPage: Int,
  pointX: Float,
  pointY: Float,
): Boolean {
  return when (
    val hit = context.editor.interactiveHitTest(page = pointPage, x = pointX, y = pointY)
  ) {
    is InteractiveHit.FoldTitle -> {
      val onText = hit.textRect?.contains(pointX, pointY) == true
      if (onText) {
        false
      } else {
        context.editor.enqueue(Message.View(ViewOp.ToggleFold(id = hit.id)))
        true
      }
    }
    is InteractiveHit.CalloutIcon -> {
      context.editor.enqueue(
        Message.Node(
          NodeOp.SetAttrs(id = hit.id, attrs = PlainNode.Callout(variant = hit.nextVariant))
        )
      )
      true
    }
    else -> false
  }
}

private fun Rect.contains(x: Float, y: Float): Boolean =
  x >= this.x && x <= this.x + width && y >= this.y && y <= this.y + height
