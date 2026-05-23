package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.canApply

internal class EditorPinchGesture {
  private val activePointers = mutableMapOf<Long, Offset>()
  var isPinching: Boolean = false
    private set

  val pointerCount: Int
    get() = activePointers.size

  fun addPointer(pointerId: Long, position: Offset) {
    activePointers[pointerId] = position
  }

  fun hasPointer(pointerId: Long): Boolean = pointerId in activePointers

  fun updatePointer(pointerId: Long, position: Offset) {
    if (pointerId in activePointers) {
      activePointers[pointerId] = position
    }
  }

  fun removePointer(pointerId: Long) {
    activePointers.remove(pointerId)
  }

  fun currentDistance(): Float? {
    val points = activePointers.values.take(2)
    if (points.size < 2) {
      return null
    }
    return (points[0] - points[1]).getDistance()
  }

  fun currentFocal(): Offset? {
    val points = activePointers.values.take(2)
    if (points.size < 2) {
      return null
    }
    return (points[0] + points[1]) / 2f
  }

  fun begin(): Boolean {
    if (isPinching || activePointers.size < 2) {
      return false
    }
    isPinching = true
    return true
  }

  fun end(): Boolean {
    if (!isPinching) {
      return false
    }
    isPinching = false
    return true
  }

  fun reset() {
    activePointers.clear()
    isPinching = false
  }
}

internal fun EditorPinchGesture.handlePointerDown(
  pointerId: Long,
  position: Offset,
  context: EditorGestureContext,
): Boolean {
  addPointer(pointerId = pointerId, position = position)
  if (isPinching) {
    return true
  }
  if (pointerCount < 2) {
    return false
  }
  return beginIfNeeded(context = context)
}

internal fun EditorPinchGesture.handlePointerMove(
  pointerId: Long,
  position: Offset,
  context: EditorGestureContext,
): Boolean {
  updatePointer(pointerId = pointerId, position = position)
  if (!isPinching) {
    return false
  }
  updateViewportZoomIfNeeded(context = context)
  return true
}

internal fun EditorPinchGesture.handlePointerUp(
  pointerId: Long,
  context: EditorGestureContext,
): Boolean {
  val wasPinching = isPinching
  removePointer(pointerId)
  if (wasPinching && pointerCount < 2) {
    end()
    context.semantics.viewportZoom.end()
  }
  return wasPinching
}

internal fun EditorPinchGesture.cancel(context: EditorGestureContext) {
  context.semantics.viewportZoom.end()
  reset()
}

private fun EditorPinchGesture.beginIfNeeded(context: EditorGestureContext): Boolean {
  val focal = currentFocal() ?: return false
  val distance = currentDistance() ?: return false
  if (!context.mode.canApply(EditorInteractionEvent.ViewportZoomStart)) {
    return false
  }
  if (!context.semantics.viewportZoom.beginPinch(focalPx = focal, distancePx = distance)) {
    return false
  }
  if (!begin()) {
    context.semantics.viewportZoom.end()
    return false
  }
  return true
}

private fun EditorPinchGesture.updateViewportZoomIfNeeded(context: EditorGestureContext): Boolean {
  val focal = currentFocal() ?: return false
  val distance = currentDistance() ?: return false
  return context.semantics.viewportZoom.updatePinch(focalPx = focal, distancePx = distance)
}
