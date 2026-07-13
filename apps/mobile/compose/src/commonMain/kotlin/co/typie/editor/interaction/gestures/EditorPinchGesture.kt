package co.typie.editor.interaction.gestures

import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorPinchSample
import co.typie.editor.interaction.canApply

internal class EditorPinchGesture {
  var isPinching: Boolean = false
    private set

  fun handleSample(sample: EditorPinchSample, context: EditorGestureContext): Boolean {
    if (isPinching) {
      context.semantics.viewportZoom.updatePinch(sample)
      return true
    }
    if (!context.mode.canApply(EditorInteractionEvent.ViewportZoomStart)) {
      return false
    }
    if (!context.semantics.viewportZoom.beginPinch(sample)) {
      return false
    }

    isPinching = true
    return true
  }

  fun end(context: EditorGestureContext): Boolean {
    if (!isPinching) {
      return false
    }

    isPinching = false
    context.semantics.viewportZoom.end()
    return true
  }

  fun cancel(context: EditorGestureContext) {
    context.semantics.viewportZoom.end()
    reset()
  }

  fun reset() {
    isPinching = false
  }
}
