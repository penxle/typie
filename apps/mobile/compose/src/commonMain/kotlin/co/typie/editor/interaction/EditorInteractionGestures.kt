package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.interaction.gestures.EditorDndGesture
import co.typie.editor.interaction.gestures.EditorLongPressDispatchDelayMillis
import co.typie.editor.interaction.gestures.EditorLongPressGesture
import co.typie.editor.interaction.gestures.EditorPanGesture
import co.typie.editor.interaction.gestures.EditorPinchGesture
import co.typie.editor.interaction.gestures.EditorSelectionHandleGesture
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
  val tableHandle: EditorTableHandleGesture = EditorTableHandleGesture(),
  val dnd: EditorDndGesture = EditorDndGesture(),
) {
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

    if (tapEnabled) {
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
    if (tapEnabled && tap.hasActivePointer) {
      longPress.prepare(pointerId = pointerId)
      context.effects.scheduleLongPressDispatch(
        pointerId = pointerId,
        position = position,
        dispatchAtMillis = nowMillis + EditorLongPressDispatchDelayMillis,
      )
    }
    return consumed
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

    if (tap.trackPointerMove(pointerId = pointerId, position = position, context = context)) {
      longPress.cancelPending(pointerId = pointerId)
      context.effects.cancelLongPressDispatch()
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

    if (longPress.cancelPending(pointerId = pointerId)) {
      context.effects.cancelLongPressDispatch()
    }

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
    return shouldConsumeTap || selectionConsumed
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
