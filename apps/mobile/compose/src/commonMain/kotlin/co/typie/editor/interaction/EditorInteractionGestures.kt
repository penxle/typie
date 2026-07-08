package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.interaction.gestures.EditorDndGesture
import co.typie.editor.interaction.gestures.EditorLongPressDispatchDelayMillis
import co.typie.editor.interaction.gestures.EditorLongPressGesture
import co.typie.editor.interaction.gestures.EditorPanGesture
import co.typie.editor.interaction.gestures.EditorPinchGesture
import co.typie.editor.interaction.gestures.EditorSelectionHandleGesture
import co.typie.editor.interaction.gestures.EditorSelectionHandleTableCellHandoff
import co.typie.editor.interaction.gestures.EditorTableHandleDragUpdate
import co.typie.editor.interaction.gestures.EditorTableHandleGesture
import co.typie.editor.interaction.gestures.EditorTapGesture
import co.typie.editor.interaction.gestures.cancel
import co.typie.editor.interaction.gestures.finish
import co.typie.editor.interaction.gestures.handlePointerDown
import co.typie.editor.interaction.gestures.handlePointerMove
import co.typie.editor.interaction.gestures.handlePointerUp
import co.typie.editor.interaction.gestures.handleTapTimer
import co.typie.editor.interaction.gestures.primeModeAtPointerDown
import co.typie.editor.interaction.gestures.start
import co.typie.editor.interaction.gestures.trackPointerMove
import co.typie.editor.interaction.gestures.update
import co.typie.editor.interaction.sessions.EditorDoubleTapDragSession

internal class EditorInteractionGestures(
  contextProvider: () -> EditorGestureContext,
  val tap: EditorTapGesture = EditorTapGesture(tapSlopPx = 0f),
  val doubleTapDrag: EditorDoubleTapDragSession = EditorDoubleTapDragSession(),
  val longPress: EditorLongPressGesture = EditorLongPressGesture(),
  val pan: EditorPanGesture = EditorPanGesture(),
  val pinch: EditorPinchGesture = EditorPinchGesture(),
  val selectionHandle: EditorSelectionHandleGesture =
    EditorSelectionHandleGesture(contextProvider = contextProvider),
  val dnd: EditorDndGesture = EditorDndGesture(),
) {
  val tableHandle: EditorTableHandleGesture =
    EditorTableHandleGesture(
      contextProvider = contextProvider,
      onHandoffToSelectionHandle = ::handoffTableDragToSelectionHandle,
    )

  val hasActivePointer: Boolean
    get() = tap.hasActivePointer

  val isIgnoringUntilAllPointersUp: Boolean
    get() = tap.isIgnoringUntilAllPointersUp

  fun updateTapSlop(tapSlopPx: Float) {
    tap.updateTapSlop(tapSlopPx)
  }

  fun handlePointerDown(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    tapEnabled: Boolean,
    inputModifiers: InputModifiers,
    context: EditorGestureContext,
  ): Boolean {
    tap.addPressedPointer(pointerId)
    if (pinch.isPinching && !pinch.hasPointer(pointerId)) {
      context.applyModeEvent(EditorInteractionEvent.PointerCancel)
      return true
    }

    val wasPinching = pinch.isPinching
    if (pinch.handlePointerDown(pointerId = pointerId, position = position, context = context)) {
      if (!wasPinching) {
        context.applyModeEvent(EditorInteractionEvent.ViewportZoomStart)
      }
      return true
    }

    if (tap.hasActivePointer) {
      tap.cancelActivePointerAndIgnoreUntilAllPointersUp()
      context.reduceMode(EditorInteractionEvent.PointerCancel)
      pinch.reset()
      resetPointerOwnedState(context = context)
      context.effects.cancelTapDispatch()
      context.effects.cancelLongPressDispatch()
      context.effects.enqueuePointerCancel()
      return false
    }

    val tableHandleHit = tapEnabled && tableHandle.hitTest(position)
    val selectionHandleType =
      if (tapEnabled && !tableHandleHit) selectionHandle.hitTest(position) else null
    if (tableHandleHit || selectionHandleType != null) {
      tap.clearTapHistory()
    }

    if (tapEnabled && !tableHandleHit && selectionHandleType == null) {
      longPress.primeModeAtPointerDown(position = position, context = context)
    }

    val consumed =
      tap.handlePointerDown(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        tapEnabled = tapEnabled,
        inputModifiers = inputModifiers,
        doubleTapDrag = doubleTapDrag,
        context = context,
      )
    val selectionHandleConsumed =
      selectionHandleType != null &&
        tap.hasActivePointer &&
        selectionHandle.handleDragDown(
          type = selectionHandleType,
          position = position,
          preserveTapDispatch = true,
        )
    val tableHandleConsumed =
      tableHandleHit && tap.hasActivePointer && tableHandle.handleDragDown(position = position)
    if (tableHandleConsumed) {
      tap.markTapDispatched()
      context.effects.cancelTapDispatch()
    }
    if (tapEnabled && tap.hasActivePointer && !selectionHandleConsumed && !tableHandleConsumed) {
      longPress.prepare(pointerId = pointerId)
      context.effects.scheduleLongPressDispatch(
        pointerId = pointerId,
        position = position,
        dispatchAtMillis = nowMillis + EditorLongPressDispatchDelayMillis,
      )
    }
    return consumed || selectionHandleConsumed || tableHandleConsumed
  }

  fun handlePointerMove(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    context: EditorGestureContext,
  ): Boolean {
    if (pinch.handlePointerMove(pointerId = pointerId, position = position, context = context)) {
      return true
    }

    if (longPress.isActivePointer(pointerId)) {
      return longPress.update(position = position, context = context)
    }

    val movedPastTapSlop =
      tap.trackPointerMove(pointerId = pointerId, position = position, context = context)
    if (movedPastTapSlop) {
      longPress.cancelPending(pointerId = pointerId)
      context.effects.cancelLongPressDispatch()
    }
    if (tableHandle.activeDrag) {
      return handleTableDragUpdate(position = position)
    }
    if (tableHandle.pendingDrag) {
      if (movedPastTapSlop && !tableHandle.activeDrag) {
        tableHandle.handleDragStart(position = position)
      }
      if (tableHandle.activeDrag) {
        return handleTableDragUpdate(position = position)
      }
      return true
    }
    selectionHandle.activeType?.let { type ->
      selectionHandle.tableCellHandoff(type = type, position = position)?.let { handoff ->
        return handoffSelectionDragToTableHandle(handoff)
      }
      selectionHandle.handleDragUpdate(type = type, position = position)
      return true
    }
    selectionHandle.pendingType?.let { type ->
      if (movedPastTapSlop && !selectionHandle.activeDrag) {
        selectionHandle.handleDragStart(type = type, position = position)
      }
      if (selectionHandle.activeDrag) {
        selectionHandle.handleDragUpdate(type = type, position = position)
      }
      return true
    }
    return doubleTapDrag.handlePointerMove(position = position, tap = tap, context = context)
  }

  fun handlePointerUp(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    context: EditorGestureContext,
  ): Boolean {
    if (longPress.isActivePointer(pointerId)) {
      context.effects.cancelLongPressDispatch()
      tap.onPointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        canFinish = false,
      )
      return longPress.finish(context = context)
    }

    if (pinch.isPinching) {
      if (pinch.handlePointerUp(pointerId = pointerId, context = context) && !pinch.isPinching) {
        // TODO(editor-parity): legacy seeds pan-resume with the remaining pinch pointer so the
        // viewport can continue scrolling after pinch ends. KMP pan-resume is not ported yet.
        context.applyModeEvent(EditorInteractionEvent.ViewportZoomEnd)
      }
      return true
    }
    pinch.handlePointerUp(pointerId = pointerId, context = context)

    if (tableHandle.activeDrag) {
      context.effects.cancelLongPressDispatch()
      tap.onPointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        canFinish = false,
      )
      return tableHandle.handleDragEnd()
    }

    selectionHandle.activeType?.let { type ->
      context.effects.cancelLongPressDispatch()
      tap.onPointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        canFinish = false,
      )
      return selectionHandle.handleDragEnd(type = type)
    }

    if (longPress.cancelPending(pointerId = pointerId)) {
      context.effects.cancelLongPressDispatch()
    }

    val pendingSelectionHandleType = selectionHandle.pendingType
    val pendingTableHandle = tableHandle.pendingDrag
    val shouldConsumeTap =
      tap.handlePointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        doubleTapDrag = doubleTapDrag,
        context = context,
      )
    val selectionConsumed = doubleTapDrag.endDrag(context = context)
    doubleTapDrag.cleanupAfterPointerUp(tap = tap, context = context)
    val pendingTableHandleConsumed = if (pendingTableHandle) tableHandle.handleDragEnd() else false
    val pendingSelectionHandleConsumed =
      pendingSelectionHandleType?.let { selectionHandle.handleDragEnd(type = it) } ?: false
    return shouldConsumeTap ||
      selectionConsumed ||
      pendingTableHandleConsumed ||
      pendingSelectionHandleConsumed
  }

  fun handleLongPressTimer(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    context: EditorGestureContext,
  ): Boolean {
    if (!canStartLongPress(mode = context.mode)) {
      longPress.cancelPending(pointerId = pointerId)
      return false
    }
    val started = longPress.start(pointerId = pointerId, position = position, context = context)
    if (started) {
      tap.markTapDispatched()
      if (longPress.isWordSelection) {
        tap.clearTapHistory()
      }
    }
    return started
  }

  fun handleTapTimer(nowMillis: Long, context: EditorGestureContext) {
    tap.handleTapTimer(nowMillis = nowMillis, doubleTapDrag = doubleTapDrag, context = context)
  }

  fun handleAppliedModeEvent(
    event: EditorInteractionEvent,
    previousMode: EditorInteractionMode,
    currentMode: EditorInteractionMode,
    context: EditorGestureContext,
  ) {
    if (previousMode != currentMode && currentMode == EditorInteractionMode.ViewportZooming) {
      resetPointerOwnedState(context = context)
    }
    if (event == EditorInteractionEvent.PointerCancel) {
      pinch.cancel(context = context)
      resetPointerOwnedState(context = context)
    }
    if (
      event == EditorInteractionEvent.PointerCancel || currentMode != EditorInteractionMode.Idle
    ) {
      cancelActivePointerStream(context = context)
    }
  }

  fun cancel(context: EditorGestureContext) {
    pinch.cancel(context = context)
    resetPointerOwnedState(context = context)
    cancelActivePointerStream(context = context)
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    tableHandle.resetPointerOwnedState(context = context)
    selectionHandle.resetPointerOwnedState(context = context)
    doubleTapDrag.resetPointerOwnedState(context = context)
    longPress.reset()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
    context.semantics.edgeAutoScroll.stop()
    context.semantics.selectionExpansion.reset()
  }

  fun cancelActivePointerStream(context: EditorGestureContext) {
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    if (tap.cancelActivePointerStream()) {
      context.effects.enqueuePointerCancel()
    }
  }

  private fun canStartLongPress(mode: EditorInteractionMode): Boolean =
    (mode.canApply(EditorInteractionEvent.LongPressStart) ||
      mode.canApply(EditorInteractionEvent.LongPressWordStart)) &&
      !tableHandle.dragging &&
      !doubleTapDrag.active

  private fun handleTableDragUpdate(position: Offset): Boolean =
    when (val update = tableHandle.handleDragUpdate(position = position)) {
      EditorTableHandleDragUpdate.NotConsumed -> false
      EditorTableHandleDragUpdate.Consumed -> true
      is EditorTableHandleDragUpdate.HandoffToSelectionHandle ->
        handoffTableDragToSelectionHandle(update)
    }

  private fun handoffTableDragToSelectionHandle(
    update: EditorTableHandleDragUpdate.HandoffToSelectionHandle
  ): Boolean {
    tableHandle.handleDragEnd()
    return selectionHandle.adoptTableCellDrag(
      touchPosition = update.touchPosition,
      handlePosition = update.handlePosition,
      tableId = update.tableId,
      anchor = update.anchor,
      baseSelection = update.baseSelection,
    )
  }

  private fun handoffSelectionDragToTableHandle(
    update: EditorSelectionHandleTableCellHandoff
  ): Boolean {
    if (!selectionHandle.handleDragHandoff()) {
      return false
    }
    if (
      !tableHandle.adoptSelectionHandleDrag(
        touchPosition = update.touchPosition,
        handlePosition = update.handlePosition,
        tableId = update.tableId,
        anchor = update.anchor,
        baseSelection = update.baseSelection,
      )
    ) {
      return false
    }
    return handleTableDragUpdate(position = update.touchPosition)
  }

  fun reset() {
    tap.reset()
    doubleTapDrag.reset()
    longPress.reset()
    pan.reset()
    pinch.reset()
    selectionHandle.reset()
    tableHandle.reset()
    dnd.reset()
  }
}
