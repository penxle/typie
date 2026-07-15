package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.interaction.gestures.EditorDndGesture
import co.typie.editor.interaction.gestures.EditorLongPressDispatchDelayMillis
import co.typie.editor.interaction.gestures.EditorLongPressGesture
import co.typie.editor.interaction.gestures.EditorPanGesture
import co.typie.editor.interaction.gestures.EditorPanGestureDriver
import co.typie.editor.interaction.gestures.EditorPinchGesture
import co.typie.editor.interaction.gestures.EditorSelectionHandleGesture
import co.typie.editor.interaction.gestures.EditorSelectionHandleTableCellHandoff
import co.typie.editor.interaction.gestures.EditorTableColumnResizeGesture
import co.typie.editor.interaction.gestures.EditorTableHandleDragUpdate
import co.typie.editor.interaction.gestures.EditorTableHandleGesture
import co.typie.editor.interaction.gestures.EditorTapGesture
import co.typie.editor.interaction.gestures.finish
import co.typie.editor.interaction.gestures.handlePointerDown
import co.typie.editor.interaction.gestures.handlePointerUp
import co.typie.editor.interaction.gestures.handleTapTimer
import co.typie.editor.interaction.gestures.primeModeAtPointerDown
import co.typie.editor.interaction.gestures.start
import co.typie.editor.interaction.gestures.trackPointerMove
import co.typie.editor.interaction.gestures.update
import co.typie.editor.interaction.sessions.EditorDoubleTapDragSession

internal class EditorInteractionGestures(
  contextProvider: () -> EditorGestureContext,
  private val tap: EditorTapGesture = EditorTapGesture(tapSlopPx = 0f),
  private val doubleTapDrag: EditorDoubleTapDragSession = EditorDoubleTapDragSession(),
  private val longPress: EditorLongPressGesture = EditorLongPressGesture(),
  private val pan: EditorPanGesture = EditorPanGesture(),
  private val pinch: EditorPinchGesture = EditorPinchGesture(),
  private val tableColumnResize: EditorTableColumnResizeGesture = EditorTableColumnResizeGesture(),
  private val selectionHandle: EditorSelectionHandleGesture =
    EditorSelectionHandleGesture(contextProvider = contextProvider),
  private val dnd: EditorDndGesture = EditorDndGesture(),
) {
  private val tableHandle: EditorTableHandleGesture =
    EditorTableHandleGesture(
      contextProvider = contextProvider,
      onHandoffToSelectionHandle = ::handoffTableDragToSelectionHandle,
    )

  fun updateTapSlop(tapSlopPx: Float) {
    tap.updateTapSlop(tapSlopPx)
    selectionHandle.updateDragSlop(tapSlopPx)
    tableHandle.updateDragSlop(tapSlopPx)
  }

  fun updateColumnResizeSlop(dragSlopPx: Float) {
    tableColumnResize.updateDragSlop(dragSlopPx)
  }

  fun handlePointerDown(
    pointerId: Long,
    positionInEditor: Offset?,
    positionInRoot: Offset,
    nowMillis: Long,
    tapEnabled: Boolean,
    inputModifiers: InputModifiers,
    touchPanDriver: EditorPanGestureDriver? = null,
    context: EditorGestureContext,
  ): Boolean {
    if (tap.hasActivePointer) {
      tap.cancelActivePointerStream()
      context.reduceMode(EditorInteractionEvent.PointerCancel)
      pinch.reset()
      resetPointerOwnedState(context = context)
      context.effects.cancelTapDispatch()
      context.effects.cancelLongPressDispatch()
      context.effects.enqueuePointerCancel()
      return false
    }

    if (
      touchPanDriver != null &&
        pan.prepareScrollCatch(
          pointerId = pointerId,
          position = positionInRoot,
          nowMillis = nowMillis,
          driver = touchPanDriver,
        )
    ) {
      return true
    }

    if (positionInEditor == null) {
      if (touchPanDriver != null) {
        pan.prepareFresh(
          pointerId = pointerId,
          position = positionInRoot,
          nowMillis = nowMillis,
          driver = touchPanDriver,
        )
      }
      return false
    }
    val position = positionInEditor

    val columnResizePlacement = tableColumnResize.hitTest(position = position, context = context)
    val tableHandleHit = columnResizePlacement == null && tableHandle.hitTest(position)
    val selectionHandleType =
      if (columnResizePlacement == null && !tableHandleHit) {
        selectionHandle.hitTest(position)
      } else {
        null
      }
    if (columnResizePlacement != null || tableHandleHit || selectionHandleType != null) {
      tap.clearTapHistory()
    }

    if (
      tapEnabled && columnResizePlacement == null && !tableHandleHit && selectionHandleType == null
    ) {
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
        selectionHandle.handleDragDown(
          type = selectionHandleType,
          position = position,
          preserveTapDispatch = true,
        )
    val tableHandleConsumed = tableHandleHit && tableHandle.handleDragDown(position = position)
    val columnResizeConsumed =
      columnResizePlacement != null &&
        tableColumnResize.handlePointerDown(
          pointerId = pointerId,
          position = position,
          placement = columnResizePlacement,
          context = context,
        )
    if (
      columnResizePlacement == null &&
        !tableHandleHit &&
        selectionHandleType == null &&
        touchPanDriver != null
    ) {
      pan.prepareFresh(
        pointerId = pointerId,
        position = positionInRoot,
        nowMillis = nowMillis,
        driver = touchPanDriver,
      )
    }
    if (
      tapEnabled &&
        tap.hasActivePointer &&
        !columnResizeConsumed &&
        !selectionHandleConsumed &&
        !tableHandleConsumed
    ) {
      longPress.prepare(pointerId = pointerId)
      context.effects.scheduleLongPressDispatch(
        pointerId = pointerId,
        position = position,
        dispatchAtMillis = nowMillis + EditorLongPressDispatchDelayMillis,
      )
    }
    return consumed || columnResizeConsumed || selectionHandleConsumed || tableHandleConsumed
  }

  fun handlePointerMove(
    pointerId: Long,
    positionInEditor: Offset?,
    positionInRoot: Offset,
    nowMillis: Long,
    pressed: Boolean = true,
    consumed: Boolean = false,
    context: EditorGestureContext,
  ): Boolean {
    if (positionInEditor == null) {
      val panConsumed =
        pan.update(
          pointerId = pointerId,
          position = positionInRoot,
          nowMillis = nowMillis,
          pressed = pressed,
          consumed = consumed,
          context = context,
        )
      if (panConsumed) {
        cancelTapAndLongPress(context = context)
      }
      return panConsumed
    }
    val position = positionInEditor

    if (longPress.isActivePointer(pointerId)) {
      return longPress.update(position = position, context = context)
    }

    val movedPastTapSlop =
      tap.trackPointerMove(pointerId = pointerId, position = position, context = context)
    if (movedPastTapSlop) {
      longPress.cancelPending(pointerId = pointerId)
      context.effects.cancelLongPressDispatch()
    }
    if (tableColumnResize.pending || tableColumnResize.active) {
      val wasActive = tableColumnResize.active
      val handled =
        tableColumnResize.handlePointerMove(
          pointerId = pointerId,
          position = position,
          context = context,
        )
      if (!wasActive && tableColumnResize.active) {
        cancelTapAndLongPress(context = context)
      }
      return handled
    }
    if (tableHandle.activeDrag) {
      return handleTableDragUpdate(position = position)
    }
    if (tableHandle.pendingDrag) {
      if (tableHandle.shouldStartDrag(position) && !tableHandle.activeDrag) {
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
      if (
        selectionHandle.shouldStartDrag(type = type, position = position) &&
          !selectionHandle.activeDrag
      ) {
        selectionHandle.handleDragStart(type = type, position = position)
      }
      if (selectionHandle.activeDrag) {
        selectionHandle.handleDragUpdate(type = type, position = position)
      }
      return true
    }
    val panConsumed =
      pan.update(
        pointerId = pointerId,
        position = positionInRoot,
        nowMillis = nowMillis,
        pressed = pressed,
        consumed = consumed,
        context = context,
      )
    if (panConsumed) {
      cancelTapAndLongPress(context = context)
      return true
    }
    return doubleTapDrag.handlePointerMove(position = position, tap = tap, context = context)
  }

  fun handlePointerUp(
    pointerId: Long,
    positionInEditor: Offset?,
    positionInRoot: Offset,
    nowMillis: Long,
    context: EditorGestureContext,
  ): Boolean {
    if (positionInEditor == null) {
      val panConsumed =
        pan.update(
          pointerId = pointerId,
          position = positionInRoot,
          nowMillis = nowMillis,
          pressed = false,
          consumed = false,
          context = context,
        )
      if (panConsumed || tap.hasActivePointer) {
        cancelTapAndLongPress(context = context)
      }
      return panConsumed
    }
    val position = positionInEditor

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

    if (tableColumnResize.active) {
      context.effects.cancelLongPressDispatch()
      tap.onPointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        canFinish = false,
      )
      return tableColumnResize.handlePointerUp(pointerId = pointerId, context = context)
    }

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

    if (
      pan.update(
        pointerId = pointerId,
        position = positionInRoot,
        nowMillis = nowMillis,
        pressed = false,
        consumed = false,
        context = context,
      )
    ) {
      cancelTapAndLongPress(context = context)
      return true
    }

    if (longPress.cancelPending(pointerId = pointerId)) {
      context.effects.cancelLongPressDispatch()
    }

    if (tableColumnResize.pending) {
      tableColumnResize.handlePointerUp(pointerId = pointerId, context = context)
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

  fun handlePinchSample(sample: EditorPinchSample, context: EditorGestureContext): Boolean {
    val wasPinching = pinch.isPinching
    if (!wasPinching) {
      pan.cancel(context = context)
    }
    if (!pinch.handleSample(sample = sample, context = context)) {
      return false
    }
    if (!wasPinching) {
      context.applyModeEvent(EditorInteractionEvent.ViewportZoomStart)
    }
    return true
  }

  fun endPinch(context: EditorGestureContext): Boolean {
    val ended = pinch.end(context = context)
    if (ended) {
      context.applyModeEvent(EditorInteractionEvent.ViewportZoomEnd)
    }
    return ended
  }

  fun endPinchAndResumeViewportPan(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    driver: EditorPanGestureDriver,
    context: EditorGestureContext,
  ): Boolean {
    if (!pinch.isPinching) {
      return false
    }
    endPinch(context = context)
    pan.resume(pointerId = pointerId, position = position, nowMillis = nowMillis, driver = driver)
    return true
  }

  fun beginPointerSignalZoom(context: EditorGestureContext): Boolean {
    if (!context.mode.canApply(EditorInteractionEvent.ViewportZoomStart)) {
      return false
    }
    if (!context.semantics.viewportZoom.beginPointerSignal()) {
      return false
    }
    context.applyModeEvent(EditorInteractionEvent.ViewportZoomStart)
    return true
  }

  fun updatePointerSignalZoom(
    focalInEditorPx: Offset,
    normalizedDelta: Float,
    context: EditorGestureContext,
  ): Boolean =
    context.semantics.viewportZoom.updatePointerSignal(
      focalInEditorPx = focalInEditorPx,
      normalizedDelta = normalizedDelta,
    )

  fun endPointerSignalZoom(context: EditorGestureContext) {
    context.semantics.viewportZoom.end()
    context.applyModeEvent(EditorInteractionEvent.ViewportZoomEnd)
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
      pan.cancel(context = context)
      resetPointerOwnedState(context = context)
    }
    if (event == EditorInteractionEvent.PointerCancel) {
      pan.cancel(context = context)
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
    pan.cancel(context = context)
    pinch.cancel(context = context)
    resetPointerOwnedState(context = context)
    cancelActivePointerStream(context = context)
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    tableColumnResize.cancel(context = context)
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

  private fun cancelTapAndLongPress(context: EditorGestureContext) {
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    longPress.reset()
    doubleTapDrag.resetPointerOwnedState(context = context)
    if (tap.cancelActivePointerStream()) {
      context.effects.enqueuePointerCancel()
    }
  }

  private fun canStartLongPress(mode: EditorInteractionMode): Boolean =
    (mode.canApply(EditorInteractionEvent.LongPressStart) ||
      mode.canApply(EditorInteractionEvent.LongPressWordStart)) &&
      !tableHandle.dragging &&
      !tableColumnResize.pending &&
      !tableColumnResize.active &&
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
    tableColumnResize.reset()
    selectionHandle.reset()
    tableHandle.reset()
    dnd.reset()
  }
}
