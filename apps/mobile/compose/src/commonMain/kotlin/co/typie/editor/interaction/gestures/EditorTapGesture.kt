package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorState
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.isViewportZooming
import co.typie.editor.interaction.sessions.EditorDoubleTapDragSession
import co.typie.platform.Platform

private const val EditorTapDownDelayMillis = 100L
private const val EditorTapTimerDelayMillis = 150L
internal const val EditorTapDispatchDelayMillis =
  EditorTapDownDelayMillis + EditorTapTimerDelayMillis

private const val ConsecutiveTapMaxIntervalMillis = 300L
private const val ConsecutiveTapMaxDistanceDp = 20f

internal class EditorTapGesture(
  private var tapSlopPx: Float,
  private val densityProvider: () -> Float,
  private val consecutiveTapMaxIntervalMillis: Long = ConsecutiveTapMaxIntervalMillis,
) {
  private var activeTap: ActiveTap? = null
  private var tapHistory: TapHistory? = null

  val hasActivePointer: Boolean
    get() = activeTap != null

  val activePosition: Offset?
    get() = activeTap?.downPosition

  val inputModifiersForActivePointer: InputModifiers
    get() = activeTap?.inputModifiers ?: InputModifiers()

  val tapCountForActivePointer: Int?
    get() = activeTap?.count

  val canDispatchTapTimer: Boolean
    get() = activeTap?.phase == ActiveTapPhase.Pending

  fun updateTapSlop(tapSlopPx: Float) {
    this.tapSlopPx = tapSlopPx
  }

  fun startActivePointer(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    inputModifiers: InputModifiers = InputModifiers(),
    contextMenuWasVisibleAtPointerDown: Boolean = false,
  ) {
    activeTap =
      ActiveTap(
        pointerId = pointerId,
        downPosition = position,
        inputModifiers = inputModifiers,
        count = nextTapCount(position = position, nowMillis = nowMillis),
        contextMenuWasVisibleAtPointerDown = contextMenuWasVisibleAtPointerDown,
      )
  }

  fun onPointerMove(pointerId: Long, position: Offset): Boolean {
    val tap = activeTap
    if (tap == null || tap.pointerId != pointerId) {
      return false
    }
    if ((position - tap.downPosition).getDistance() > tapSlopPx) {
      tap.phase = ActiveTapPhase.Moved
      return true
    }
    return false
  }

  fun markTapDispatched() {
    activeTap?.let { tap ->
      if (tap.phase == ActiveTapPhase.Pending) {
        tap.phase = ActiveTapPhase.Dispatched
      }
    }
  }

  fun shouldConsumePointerUp(pointerId: Long, canFinish: Boolean): Boolean {
    val tap = activeTap ?: return false
    return canFinish && tap.pointerId == pointerId && tap.phase != ActiveTapPhase.Moved
  }

  fun onPointerUp(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    canFinish: Boolean = true,
  ): Int? {
    val tap = activeTap
    if (tap == null || tap.pointerId != pointerId) {
      return null
    }
    if (!canFinish) {
      clearActivePointer()
      return null
    }

    val clickCount = if (tap.phase == ActiveTapPhase.Pending) tap.count else null
    if (tap.phase == ActiveTapPhase.Recorded) {
      recordTap(nowMillis = nowMillis, position = position, clickCount = tap.count)
    }
    clearActivePointer()
    return clickCount
  }

  fun cancelActivePointerStream(): Boolean {
    val hadActivePointer = activeTap != null
    clearActivePointer()
    return hadActivePointer
  }

  fun reset() {
    clearActivePointer()
    clearTapHistory()
  }

  private fun clearActivePointer() {
    activeTap = null
  }

  fun recordTap(nowMillis: Long, position: Offset, clickCount: Int) {
    activeTap?.let { tap ->
      if (clickCount == tap.count && tap.phase != ActiveTapPhase.Moved) {
        tap.phase = ActiveTapPhase.Recorded
      }
    }
    if (clickCount >= 3) {
      clearTapHistory()
      return
    }
    tapHistory = TapHistory(completedAtMillis = nowMillis, position = position, count = clickCount)
  }

  fun clearTapHistory() {
    tapHistory = null
  }

  fun shouldOpenContextMenuForCurrentTap(): Boolean {
    val tap = activeTap ?: return false
    return !tap.contextMenuWasVisibleAtPointerDown
  }

  fun nextTapCount(position: Offset, nowMillis: Long): Int {
    val previousTap = tapHistory ?: return 1
    return if (
      nowMillis - previousTap.completedAtMillis < consecutiveTapMaxIntervalMillis &&
        (position - previousTap.position).getDistance() <
          ConsecutiveTapMaxDistanceDp * densityProvider()
    ) {
      previousTap.count + 1
    } else {
      1
    }
  }

  private class ActiveTap(
    val pointerId: Long,
    val downPosition: Offset,
    val inputModifiers: InputModifiers,
    val count: Int,
    val contextMenuWasVisibleAtPointerDown: Boolean,
    var phase: ActiveTapPhase = ActiveTapPhase.Pending,
  )

  private enum class ActiveTapPhase {
    Pending,
    Dispatched,
    Recorded,
    Moved,
  }

  private data class TapHistory(val completedAtMillis: Long, val position: Offset, val count: Int)
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
  if (!tapEnabled || context.mode.isViewportZooming) {
    return false
  }

  startActivePointer(
    pointerId = pointerId,
    position = position,
    nowMillis = nowMillis,
    inputModifiers = inputModifiers,
    contextMenuWasVisibleAtPointerDown = context.semantics.contextMenu.visible,
  )
  context.semantics.contextMenu.hide()
  if (tapCountForActivePointer == 2) {
    markTapDispatched()
    context.effects.cancelTapDispatch()
    dispatchTap(
      position = position,
      nowMillis = nowMillis,
      clickCount = 2,
      inputModifiers = inputModifiers,
      shouldOpenContextMenu = shouldOpenContextMenuForCurrentTap(),
      doubleTapDrag = doubleTapDrag,
      context = context,
    ) {
      doubleTapDrag.prepareForDrag(position = position, tap = this, context = context)
    }
    return true
  }

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
  val shouldOpenContextMenu = shouldOpenContextMenuForCurrentTap()
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
      shouldOpenContextMenu = shouldOpenContextMenu,
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
  val clickCount = tapCountForActivePointer ?: return
  val point = context.geometry.resolvePoint(positionInNode = position)
  val hitSelection =
    point != null &&
      point.page >= 0 &&
      context.editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (clickCount == 1 && context.platform != Platform.Android && hitSelection) {
    markTapDispatched()
    if (shouldOpenContextMenuForCurrentTap()) {
      context.semantics.contextMenu.show(context.editor.state)
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
    shouldOpenContextMenu = shouldOpenContextMenuForCurrentTap(),
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
  shouldOpenContextMenu: Boolean,
  doubleTapDrag: EditorDoubleTapDragSession,
  context: EditorGestureContext,
  beforeLaunch: () -> Unit,
): Boolean {
  val point = context.geometry.resolvePoint(positionInNode = position) ?: return false
  if (context.mode.isViewportZooming || point.page < 0) {
    return false
  }
  val editor = context.editor
  if (context.semantics.interactiveHit.handleTap(editor = editor, point = point)) {
    return true
  }
  recordTap(nowMillis = nowMillis, position = position, clickCount = clickCount)
  val wasFocused = context.isFocused
  context.effects.requestFocus(editor)
  if (wasFocused) {
    context.effects.requestSoftwareKeyboard()
  }
  val hitExistingSelectionAtTap =
    editor.selectionHitTest(page = point.page, x = point.x, y = point.y)
  if (clickCount == 1 && context.platform != Platform.Android && hitExistingSelectionAtTap) {
    if (shouldOpenContextMenu) {
      context.semantics.contextMenu.show(editor.state)
    }
    return false
  }
  val previousCursor = editor.cursor
  beforeLaunch()
  val tap = this
  val beforeCommit: (EditorState) -> Unit = { snapshot ->
    if (clickCount == 1) {
      when {
        !snapshot.selection.isCollapsed() -> {
          if (!hitExistingSelectionAtTap) {
            context.semantics.contextMenu.show(snapshot)
            context.effects.requestCurrentSelectionHead(version = snapshot.version)
          } else {
            context.semantics.contextMenu.hide()
          }
        }
        isSameCursorTap(previousCursor, snapshot) -> {
          if (wasFocused && shouldOpenContextMenu) {
            context.semantics.contextMenu.show(snapshot)
          }
        }
        else -> {
          context.semantics.contextMenu.hide()
          if (snapshot.cursor != null) {
            context.effects.requestCurrentSelectionHead(version = snapshot.version)
          }
        }
      }
    }
  }
  val afterDispatch: (Boolean) -> Unit = { dispatched ->
    if (dispatched) {
      when (clickCount) {
        2 -> doubleTapDrag.onWordSelectionCommitted(tap = tap, context = context)
        3 -> {
          if (!editor.selection.isCollapsed()) {
            context.semantics.contextMenu.show(editor.state)
          }
        }
      }
    }
  }
  when {
    clickCount <= 1 && inputModifiers.shift ->
      context.semantics.pointSelection.launchSelectionExtension(
        editor = editor,
        point = point,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
    clickCount <= 1 ->
      context.semantics.pointSelection.launchCursorMove(
        editor = editor,
        point = point,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
    clickCount == 2 ->
      context.semantics.pointSelection.launchUnitSelection(
        editor = editor,
        point = point,
        unit = SelectionPointUnit.Word,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
    else ->
      context.semantics.pointSelection.launchUnitSelection(
        editor = editor,
        point = point,
        unit = SelectionPointUnit.Paragraph,
        beforeCommit = beforeCommit,
        afterDispatch = afterDispatch,
      )
  }
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
