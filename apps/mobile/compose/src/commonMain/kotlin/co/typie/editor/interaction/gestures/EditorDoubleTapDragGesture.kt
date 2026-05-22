package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset

private const val EditorDoubleTapDragStartThresholdPx = 4f

internal class EditorDoubleTapDragGesture {
  private var phase = EditorDoubleTapDragPhase.Idle
  var startPosition: Offset? = null
    private set

  val active: Boolean
    get() = phase != EditorDoubleTapDragPhase.Idle

  val pending: Boolean
    get() = phase == EditorDoubleTapDragPhase.Pending

  val dragging: Boolean
    get() = phase == EditorDoubleTapDragPhase.Dragging

  fun prepare(startPosition: Offset) {
    this.startPosition = startPosition
    phase = EditorDoubleTapDragPhase.Pending
  }

  fun canStart(position: Offset): Boolean {
    val startPosition = startPosition ?: return false
    return pending &&
      (position - startPosition).getDistance() >= EditorDoubleTapDragStartThresholdPx
  }

  fun begin(): Boolean {
    if (!pending) {
      return false
    }
    phase = EditorDoubleTapDragPhase.Dragging
    return true
  }

  fun stop(): Boolean {
    val wasActive = active
    startPosition = null
    phase = EditorDoubleTapDragPhase.Idle
    return wasActive
  }

  fun reset() {
    stop()
  }
}

private enum class EditorDoubleTapDragPhase {
  Idle,
  Pending,
  Dragging,
}
