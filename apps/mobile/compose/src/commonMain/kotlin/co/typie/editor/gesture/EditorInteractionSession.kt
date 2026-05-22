package co.typie.editor.gesture

import androidx.compose.ui.geometry.Offset

internal class EditorInteractionSession(
  tapSlopPx: Float,
  private val core: EditorInteractionCore = EditorInteractionCore(),
  private val gestures: EditorInteractionGestureSet =
    EditorInteractionGestureSet(tap = EditorTapGesture(tapSlopPx = tapSlopPx)),
) {
  private var mode = EditorInteractionMode.Idle

  val isIgnoringUntilAllPointersUp: Boolean
    get() = gestures.tap.isIgnoringUntilAllPointersUp

  val hasActivePointer: Boolean
    get() = gestures.tap.hasActivePointer

  val interactionMode: EditorInteractionMode
    get() = mode

  fun updateTapSlop(tapSlopPx: Float) {
    gestures.updateTapSlop(tapSlopPx)
  }

  fun applyEvent(event: EditorInteractionEvent): EditorInteractionPointerResult {
    mode = core.reduce(previous = mode, event = event)
    return if (
      event == EditorInteractionEvent.PointerCancel || mode != EditorInteractionMode.Idle
    ) {
      gestures.tap.cancelActivePointerStream()
    } else {
      EditorInteractionPointerResult()
    }
  }

  fun canDispatchTap(page: Int): Boolean =
    core.decide(
      command = EditorInteractionCommand.TapDispatch(page = page),
      runtime = runtimeRead(),
    )

  fun onPointerDown(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
  ): EditorInteractionPointerResult =
    gestures.tap.onPointerDown(pointerId = pointerId, position = position) {
      core.decide(command = EditorInteractionCommand.TapDown, runtime = runtimeRead())
    }

  fun onPointerMove(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
  ): EditorInteractionPointerResult =
    gestures.tap.onPointerMove(pointerId = pointerId, position = position)

  fun onTapTimer(nowMillis: Long): EditorInteractionTapDispatch? =
    gestures.tap.onTapTimer(nowMillis)

  fun onPointerUp(
    pointerId: Long,
    position: Offset,
    nowMillis: Long,
  ): EditorInteractionPointerResult =
    gestures.tap.onPointerUp(pointerId = pointerId, position = position, nowMillis = nowMillis) {
      core.decide(command = EditorInteractionCommand.TapUp, runtime = runtimeRead())
    }

  fun cancel(): EditorInteractionPointerResult {
    mode = core.reduce(previous = mode, event = EditorInteractionEvent.PointerCancel)
    return gestures.tap.cancelActivePointerStream()
  }

  fun reset() {
    mode = EditorInteractionMode.Idle
    gestures.reset()
  }

  private fun runtimeRead(): EditorInteractionRuntimeRead = gestures.runtimeRead(mode)
}
