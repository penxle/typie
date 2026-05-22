package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.gestures.EditorTapDispatchDelayMillis
import co.typie.editor.interaction.gestures.EditorTapGesture
import co.typie.editor.interaction.semantics.dispatchPrimaryClick
import co.typie.editor.interaction.semantics.dispatchSelectionExtension
import co.typie.editor.interaction.semantics.hasRangeSelection
import co.typie.editor.interaction.semantics.isSelectionHit

internal interface EditorInteractionControllerHost {
  fun resolvePoint(positionInNode: Offset): PagePoint?

  fun scheduleTapDispatch(dispatchAtMillis: Long)

  fun cancelTapDispatch()

  fun launchInteraction(block: suspend () -> Unit)

  fun requestFocus(editor: Editor): Boolean

  fun enqueuePointerCancel()

  fun requestCurrentCursorLine(version: Long)
}

internal class EditorInteractionController(
  private val editorProvider: () -> Editor,
  private val host: EditorInteractionControllerHost,
  private val core: EditorInteractionCore = EditorInteractionCore(),
  private val gestures: EditorInteractionGestures =
    EditorInteractionGestures(tap = EditorTapGesture(tapSlopPx = 0f)),
  private val semantics: EditorInteractionSemantics = EditorInteractionSemantics(),
) {
  private var mode = EditorInteractionMode.Idle
  private var pendingSelectionExtensionPosition: Offset? = null

  val interactionMode: EditorInteractionMode
    get() = mode

  val hasActivePointer: Boolean
    get() = gestures.tap.hasActivePointer

  val isIgnoringUntilAllPointersUp: Boolean
    get() = gestures.tap.isIgnoringUntilAllPointersUp

  fun updateTapSlop(tapSlopPx: Float) {
    gestures.updateTapSlop(tapSlopPx)
  }

  fun onPointerDown(pointerId: Long, position: Offset, nowMillis: Long): Boolean {
    val tap = gestures.tap
    tap.addPressedPointer(pointerId)
    if (!decide(EditorInteractionCommand.TapDown) || tap.isIgnoringUntilAllPointersUp) {
      return false
    }

    if (tap.hasActivePointer) {
      tap.cancelActivePointerAndIgnoreUntilAllPointersUp()
      transition(EditorInteractionEvent.PointerCancel)
      resetPointerOwnedState()
      host.cancelTapDispatch()
      host.enqueuePointerCancel()
      return false
    }

    tap.startActivePointer(pointerId = pointerId, position = position)
    if (tap.nextTapCount(position = position, nowMillis = nowMillis) == 2) {
      tap.markTapDispatched()
      host.cancelTapDispatch()
      dispatchTap(position = position, nowMillis = nowMillis, clickCount = 2) {
        if (prepareDoubleTapDrag(position)) {
          semantics.selectionExpansion.awaitWordSelectionCommit()
        }
      }
      return true
    }

    tap.markTapPending()
    host.scheduleTapDispatch(dispatchAtMillis = nowMillis + EditorTapDispatchDelayMillis)
    return false
  }

  fun onPointerMove(pointerId: Long, position: Offset, nowMillis: Long): Boolean {
    if (gestures.tap.onPointerMove(pointerId = pointerId, position = position)) {
      host.cancelTapDispatch()
    }
    return handleSelectionPointerMove(position = position)
  }

  fun onPointerUp(pointerId: Long, position: Offset, nowMillis: Long): Boolean {
    val canFinishTap = decide(EditorInteractionCommand.TapUp)
    val shouldConsumeTap =
      gestures.tap.shouldConsumePointerUp(pointerId = pointerId, canFinish = canFinishTap)
    val clickCount =
      gestures.tap.onPointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        canFinish = canFinishTap,
      )
    if (!canFinishTap) {
      host.cancelTapDispatch()
    }
    clickCount?.let {
      dispatchTap(position = position, nowMillis = nowMillis, clickCount = it, beforeLaunch = {})
    }

    val selectionConsumed = endDoubleTapDrag()
    if (!gestures.tap.hasActivePointer) {
      if (!hasDeferredSelectionExtension()) {
        resetSelectionExtensionState()
      }
    }
    return shouldConsumeTap || selectionConsumed
  }

  fun onTapTimer(nowMillis: Long) {
    val position = gestures.tap.activePosition ?: return
    if (!gestures.tap.canDispatchTapTimer) {
      return
    }
    val clickCount = gestures.tap.nextTapCount(position = position, nowMillis = nowMillis)
    if (clickCount == 1 && isSelectionHit(position)) {
      gestures.tap.markTapDispatched()
      return
    }
    if (clickCount == 1 && semantics.cursorMove.hasRangeSelection(editorProvider())) {
      return
    }
    gestures.tap.markTapDispatched()
    dispatchTap(
      position = position,
      nowMillis = nowMillis,
      clickCount = clickCount,
      beforeLaunch = {},
    )
  }

  fun applyEvent(event: EditorInteractionEvent) {
    val previousMode = mode
    transition(event)
    cleanupForModeTransition(previousMode = previousMode, currentMode = mode)
    if (event == EditorInteractionEvent.PointerCancel) {
      resetPointerOwnedState()
    }
    if (event == EditorInteractionEvent.PointerCancel || mode != EditorInteractionMode.Idle) {
      cancelActivePointerStream()
    }
  }

  fun cancel() {
    transition(EditorInteractionEvent.PointerCancel)
    resetPointerOwnedState()
    cancelActivePointerStream()
  }

  fun reset() {
    mode = EditorInteractionMode.Idle
    pendingSelectionExtensionPosition = null
    gestures.reset()
    semantics.reset()
  }

  private fun dispatchTap(
    position: Offset,
    nowMillis: Long,
    clickCount: Int,
    beforeLaunch: () -> Unit,
  ): Boolean {
    val point = host.resolvePoint(positionInNode = position) ?: return false
    if (!decide(EditorInteractionCommand.TapDispatch(page = point.page))) {
      return false
    }
    gestures.tap.recordTap(nowMillis = nowMillis, position = position, clickCount = clickCount)
    val editor = editorProvider()
    if (clickCount == 1 && semantics.cursorMove.isSelectionHit(editor, point)) {
      return false
    }
    val previousCursor = editor.cursor
    host.requestFocus(editor)
    beforeLaunch()
    host.launchInteraction {
      val dispatched =
        semantics.cursorMove.dispatchPrimaryClick(
          editor = editor,
          point = point,
          clickCount = clickCount,
          beforeCommit = { snapshot ->
            if (clickCount == 1 && shouldRequestSingleTapBringIntoView(previousCursor, snapshot)) {
              host.requestCurrentCursorLine(version = snapshot.version)
            }
          },
        )
      if (dispatched && clickCount == 2) {
        semantics.selectionExpansion.markWordSelectionCommitted()
        flushPendingSelectionExtension()
        if (!gestures.tap.hasActivePointer && !gestures.doubleTapDrag.active) {
          resetSelectionExtensionState()
        }
      }
    }
    return true
  }

  private fun prepareDoubleTapDrag(position: Offset): Boolean {
    if (!decide(EditorInteractionCommand.DoubleTapPrepareDrag)) {
      return false
    }
    host.cancelTapDispatch()
    gestures.tap.markTapDispatched()
    semantics.selectionExpansion.reset()
    gestures.doubleTapDrag.prepare(startPosition = position)
    return true
  }

  private fun handleSelectionPointerMove(position: Offset): Boolean {
    val doubleTapDrag = gestures.doubleTapDrag
    if (doubleTapDrag.pending) {
      val startPosition = doubleTapDrag.startPosition
      if (startPosition != null && doubleTapDrag.canStart(position)) {
        if (startDoubleTapDrag()) {
          updateDoubleTapDragSelection(position)
        }
      }
      return true
    }

    if (doubleTapDrag.dragging) {
      updateDoubleTapDragSelection(position)
      return true
    }

    return false
  }

  private fun startDoubleTapDrag(): Boolean {
    if (!decide(EditorInteractionCommand.DoubleTapStartDrag)) {
      return false
    }
    host.cancelTapDispatch()
    gestures.tap.markTapDispatched()
    if (!gestures.doubleTapDrag.begin()) {
      return false
    }
    if (!decide(EditorInteractionCommand.DoubleTapBeginSelecting)) {
      gestures.doubleTapDrag.stop()
      return false
    }
    transition(EditorInteractionEvent.DoubleTapDragStart)
    return true
  }

  private fun updateDoubleTapDragSelection(position: Offset): Boolean {
    if (
      !decide(
        EditorInteractionCommand.DoubleTapUpdateSelection(
          localPosition = position,
          dragStartPosition = gestures.doubleTapDrag.startPosition,
        )
      )
    ) {
      return false
    }

    return extendDoubleTapDragSelection(position)
  }

  private fun extendDoubleTapDragSelection(position: Offset): Boolean {
    val point = host.resolvePoint(positionInNode = position) ?: return false
    val editor = editorProvider()
    val context = semantics.selectionExpansion.context(editor)
    if (context == null) {
      if (semantics.selectionExpansion.isAwaitingWordSelectionCommit) {
        pendingSelectionExtensionPosition = position
      }
      return false
    }
    if (
      !decide(
        EditorInteractionCommand.DoubleTapExtendSelection(
          page = point.page,
          hasSelectionContext = true,
        )
      )
    ) {
      return false
    }
    if (editor.dispatchSelectionExtension(point = point, context = context)) {
      pendingSelectionExtensionPosition = null
      return true
    }
    return false
  }

  private fun endDoubleTapDrag(): Boolean {
    val wasActive = gestures.doubleTapDrag.active
    val wasDragging = gestures.doubleTapDrag.dragging
    val wasPending = gestures.doubleTapDrag.pending
    if (!gestures.doubleTapDrag.stop()) {
      return false
    }
    if (wasDragging) {
      transition(EditorInteractionEvent.DoubleTapDragEnd)
    } else if (wasPending) {
      // TODO(editor-parity): legacy shows the selection context menu when a double tap selects a
      // range but never crosses the drag threshold. KMP does not host that menu state yet.
    }
    return wasActive
  }

  private fun isSelectionHit(position: Offset): Boolean {
    val point = host.resolvePoint(positionInNode = position) ?: return true
    return point.page < 0 || semantics.cursorMove.isSelectionHit(editorProvider(), point)
  }

  private fun flushPendingSelectionExtension() {
    val position = pendingSelectionExtensionPosition ?: return
    pendingSelectionExtensionPosition = null
    extendDoubleTapDragSelection(position = position)
  }

  private fun hasDeferredSelectionExtension(): Boolean =
    pendingSelectionExtensionPosition != null &&
      semantics.selectionExpansion.isAwaitingWordSelectionCommit

  private fun resetSelectionExtensionState() {
    pendingSelectionExtensionPosition = null
    semantics.selectionExpansion.reset()
  }

  private fun resetPointerOwnedState() {
    resetSelectionExtensionState()
    gestures.doubleTapDrag.reset()
  }

  private fun cleanupForModeTransition(
    previousMode: EditorInteractionMode,
    currentMode: EditorInteractionMode,
  ) {
    if (previousMode == currentMode) {
      return
    }
    if (currentMode == EditorInteractionMode.Pinching) {
      resetPointerOwnedState()
    }
  }

  private fun cancelActivePointerStream() {
    host.cancelTapDispatch()
    if (gestures.tap.cancelActivePointerStream()) {
      host.enqueuePointerCancel()
    }
  }

  private fun runtimeRead(): EditorInteractionRuntimeRead = gestures.runtimeRead(mode)

  private fun decide(command: EditorInteractionCommand): Boolean =
    core.decide(command = command, runtime = runtimeRead())

  private fun transition(event: EditorInteractionEvent) {
    mode = core.reduce(previous = mode, event = event)
  }
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
