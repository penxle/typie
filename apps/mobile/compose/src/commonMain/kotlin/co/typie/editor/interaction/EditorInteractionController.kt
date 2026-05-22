package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.interaction.gestures.EditorTapGesture
import co.typie.editor.interaction.gestures.cancel
import co.typie.editor.interaction.gestures.handlePointerDown
import co.typie.editor.interaction.gestures.handlePointerMove
import co.typie.editor.interaction.gestures.handlePointerUp
import co.typie.editor.interaction.gestures.handleTapTimer
import co.typie.editor.interaction.gestures.trackPointerMove

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
  private val gestureContext =
    object : EditorGestureContext {
      override val editor: Editor
        get() = editorProvider()

      override val semantics: EditorInteractionSemantics
        get() = this@EditorInteractionController.semantics

      override fun can(command: EditorInteractionCommand): Boolean = decide(command)

      override fun transition(event: EditorInteractionEvent) {
        this@EditorInteractionController.transition(event)
      }

      override fun resolvePoint(positionInNode: Offset): PagePoint? =
        host.resolvePoint(positionInNode = positionInNode)

      override fun cancelTapDispatch() {
        host.cancelTapDispatch()
      }

      override fun scheduleTapDispatch(dispatchAtMillis: Long) {
        host.scheduleTapDispatch(dispatchAtMillis = dispatchAtMillis)
      }

      override fun launchInteraction(block: suspend () -> Unit) {
        host.launchInteraction(block)
      }

      override fun requestFocus(editor: Editor): Boolean = host.requestFocus(editor)

      override fun requestCurrentCursorLine(version: Long) {
        host.requestCurrentCursorLine(version = version)
      }
    }
  val interactionMode: EditorInteractionMode
    get() = mode

  val hasActivePointer: Boolean
    get() = gestures.tap.hasActivePointer

  val isIgnoringUntilAllPointersUp: Boolean
    get() = gestures.tap.isIgnoringUntilAllPointersUp

  fun updateTapSlop(tapSlopPx: Float) {
    gestures.updateTapSlop(tapSlopPx)
  }

  fun can(command: EditorInteractionCommand): Boolean = decide(command)

  fun onPointerDown(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
    tapEnabled: Boolean = true,
  ): Boolean {
    val tap = gestures.tap
    tap.addPressedPointer(pointerId)
    if (gestures.pinch.isPinching && !gestures.pinch.hasPointer(pointerId)) {
      applyEvent(EditorInteractionEvent.PointerCancel)
      return true
    }

    val wasPinching = gestures.pinch.isPinching
    if (
      gestures.pinch.handlePointerDown(
        pointerId = pointerId,
        position = position,
        context = gestureContext,
      )
    ) {
      if (!wasPinching) {
        applyEvent(EditorInteractionEvent.ViewportZoomStart)
      }
      return true
    }

    if (tap.hasActivePointer) {
      tap.cancelActivePointerAndIgnoreUntilAllPointersUp()
      transition(EditorInteractionEvent.PointerCancel)
      gestures.pinch.reset()
      resetPointerOwnedState()
      host.cancelTapDispatch()
      host.enqueuePointerCancel()
      return false
    }

    return tap.handlePointerDown(
      pointerId = pointerId,
      position = position,
      nowMillis = nowMillis,
      tapEnabled = tapEnabled,
      doubleTapDrag = gestures.doubleTapDrag,
      context = gestureContext,
    )
  }

  fun onPointerMove(pointerId: Long, position: Offset, nowMillis: Long): Boolean {
    if (
      gestures.pinch.handlePointerMove(
        pointerId = pointerId,
        position = position,
        context = gestureContext,
      )
    ) {
      return true
    }

    gestures.tap.trackPointerMove(
      pointerId = pointerId,
      position = position,
      context = gestureContext,
    )
    return gestures.doubleTapDrag.handlePointerMove(
      position = position,
      tap = gestures.tap,
      context = gestureContext,
    )
  }

  fun onPointerUp(pointerId: Long, position: Offset, nowMillis: Long): Boolean {
    if (gestures.pinch.isPinching) {
      if (
        gestures.pinch.handlePointerUp(pointerId = pointerId, context = gestureContext) &&
          !gestures.pinch.isPinching
      ) {
        // TODO(editor-parity): legacy seeds pan-resume with the remaining pinch pointer so the
        // viewport can continue scrolling after pinch ends. KMP pan-resume is not ported yet.
        applyEvent(EditorInteractionEvent.ViewportZoomEnd)
      }
      return true
    }
    gestures.pinch.handlePointerUp(pointerId = pointerId, context = gestureContext)

    val shouldConsumeTap =
      gestures.tap.handlePointerUp(
        pointerId = pointerId,
        position = position,
        nowMillis = nowMillis,
        doubleTapDrag = gestures.doubleTapDrag,
        context = gestureContext,
      )
    val selectionConsumed = gestures.doubleTapDrag.endDrag(context = gestureContext)
    gestures.doubleTapDrag.cleanupAfterPointerUp(tap = gestures.tap, context = gestureContext)
    return shouldConsumeTap || selectionConsumed
  }

  fun onTapTimer(nowMillis: Long) {
    gestures.tap.handleTapTimer(
      nowMillis = nowMillis,
      doubleTapDrag = gestures.doubleTapDrag,
      context = gestureContext,
    )
  }

  fun applyEvent(event: EditorInteractionEvent) {
    val previousMode = mode
    transition(event)
    cleanupForModeTransition(previousMode = previousMode, currentMode = mode)
    if (event == EditorInteractionEvent.PointerCancel) {
      gestures.pinch.cancel(context = gestureContext)
      resetPointerOwnedState()
    }
    if (event == EditorInteractionEvent.PointerCancel || mode != EditorInteractionMode.Idle) {
      cancelActivePointerStream()
    }
  }

  fun cancel() {
    transition(EditorInteractionEvent.PointerCancel)
    gestures.pinch.cancel(context = gestureContext)
    resetPointerOwnedState()
    cancelActivePointerStream()
  }

  fun reset() {
    mode = EditorInteractionMode.Idle
    gestures.reset()
    semantics.reset()
  }

  private fun resetPointerOwnedState() {
    gestures.doubleTapDrag.resetPointerOwnedState(context = gestureContext)
  }

  private fun cleanupForModeTransition(
    previousMode: EditorInteractionMode,
    currentMode: EditorInteractionMode,
  ) {
    if (previousMode == currentMode) {
      return
    }
    if (currentMode == EditorInteractionMode.ViewportZooming) {
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
