package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ext.isCollapsed
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.gestures.EditorTapGesture
import co.typie.editor.interaction.isViewportZooming
import co.typie.editor.interaction.semantics.dispatchSelectionExtension

private const val EditorDoubleTapDragStartThresholdDp = 4f

internal class EditorDoubleTapDragSession {
  private var phase = EditorDoubleTapDragPhase.Idle
  private var pendingSelectionExtensionPosition: Offset? = null
  private var startPosition: Offset? = null
  private var startThresholdPx = 0f

  val active: Boolean
    get() = phase != EditorDoubleTapDragPhase.Idle

  val pending: Boolean
    get() = phase == EditorDoubleTapDragPhase.Pending

  val dragging: Boolean
    get() = phase == EditorDoubleTapDragPhase.Dragging

  fun prepareForDrag(
    position: Offset,
    tap: EditorTapGesture,
    context: EditorGestureContext,
  ): Boolean {
    if (context.mode.isViewportZooming) {
      return false
    }
    context.effects.cancelTapDispatch()
    tap.markTapDispatched()
    context.semantics.selectionExpansion.reset()
    context.semantics.selectionExpansion.awaitWordSelectionCommit()
    context.uiState.contextMenu.hide()
    context.effects.setScrollGestureLocked(true)
    startPosition = position
    startThresholdPx = EditorDoubleTapDragStartThresholdDp * context.geometry.density
    phase = EditorDoubleTapDragPhase.Pending
    return true
  }

  fun handlePointerMove(
    position: Offset,
    tap: EditorTapGesture,
    context: EditorGestureContext,
  ): Boolean {
    if (pending) {
      if (canStart(position) && start(tap = tap, context = context)) {
        updateSelection(position = position, context = context)
      }
      return true
    }

    if (dragging) {
      updateSelection(position = position, context = context)
      return true
    }

    return false
  }

  fun endDrag(context: EditorGestureContext): Boolean {
    val wasActive = active
    val wasDragging = dragging
    val wasPending = pending
    if (!stop()) {
      return false
    }
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    if (wasDragging) {
      if (context.mode.canApply(EditorInteractionEvent.DoubleTapDragEnd)) {
        context.reduceMode(EditorInteractionEvent.DoubleTapDragEnd)
      }
      if (!context.editor.selection.isCollapsed()) {
        context.uiState.contextMenu.show(context.editor.state)
      }
    } else if (wasPending) {
      if (!context.editor.selection.isCollapsed()) {
        context.uiState.contextMenu.show(context.editor.state)
      }
    }
    context.semantics.magnifier.hide()
    return wasActive
  }

  fun onWordSelectionCommitted(tap: EditorTapGesture, context: EditorGestureContext) {
    context.semantics.selectionExpansion.markWordSelectionCommitted()
    flushPendingSelectionExtension(context = context)
    if (!tap.hasActivePointer && !active) {
      if (!context.editor.selection.isCollapsed()) {
        context.uiState.contextMenu.show(context.editor.state)
      }
      resetSelectionExtensionState(context = context)
    }
  }

  fun cleanupAfterPointerUp(tap: EditorTapGesture, context: EditorGestureContext) {
    if (!tap.hasActivePointer && !hasDeferredSelectionExtension(context = context)) {
      resetSelectionExtensionState(context = context)
    }
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    context.effects.setScrollGestureLocked(false)
    context.semantics.edgeAutoScroll.stop()
    resetSelectionExtensionState(context = context)
    reset()
  }

  fun reset() {
    pendingSelectionExtensionPosition = null
    stop()
  }

  private fun canStart(position: Offset): Boolean {
    val startPosition = startPosition ?: return false
    return pending && (position - startPosition).getDistance() >= startThresholdPx
  }

  private fun start(tap: EditorTapGesture, context: EditorGestureContext): Boolean {
    if (!context.mode.canApply(EditorInteractionEvent.DoubleTapDragStart)) {
      return false
    }
    context.effects.cancelTapDispatch()
    tap.markTapDispatched()
    if (!begin()) {
      return false
    }
    context.reduceMode(EditorInteractionEvent.DoubleTapDragStart)
    if (context.mode != EditorInteractionMode.DoubleTapSelecting) {
      stop()
      context.effects.setScrollGestureLocked(false)
      return false
    }
    return true
  }

  private fun begin(): Boolean {
    if (!pending) {
      return false
    }
    phase = EditorDoubleTapDragPhase.Dragging
    return true
  }

  private fun stop(): Boolean {
    val wasActive = active
    startPosition = null
    startThresholdPx = 0f
    phase = EditorDoubleTapDragPhase.Idle
    return wasActive
  }

  private fun updateSelection(position: Offset, context: EditorGestureContext): Boolean {
    if (
      context.mode.isViewportZooming ||
        !dragging ||
        (startPosition != null && (position - startPosition!!).getDistance() < startThresholdPx)
    ) {
      return false
    }

    return extendSelection(position = position, context = context)
  }

  private fun extendSelection(position: Offset, context: EditorGestureContext): Boolean {
    context.semantics.edgeAutoScroll.trackSelectionExpansion(
      edgePosition = position,
      dispatchPosition = position,
      context = context,
    )
    val point = context.geometry.resolvePoint(positionInNode = position) ?: return false
    val editor = context.editor
    val selectionContext = context.semantics.selectionExpansion.context(editor)
    if (selectionContext == null) {
      if (context.semantics.selectionExpansion.isAwaitingWordSelectionCommit) {
        pendingSelectionExtensionPosition = position
      }
      return false
    }
    if (point.page < 0) {
      return false
    }
    if (editor.dispatchSelectionExtension(point = point, context = selectionContext)) {
      pendingSelectionExtensionPosition = null
      context.semantics.magnifier.show(position)
      return true
    }
    return false
  }

  private fun flushPendingSelectionExtension(context: EditorGestureContext) {
    val position = pendingSelectionExtensionPosition ?: return
    pendingSelectionExtensionPosition = null
    extendSelection(position = position, context = context)
  }

  private fun hasDeferredSelectionExtension(context: EditorGestureContext): Boolean =
    pendingSelectionExtensionPosition != null &&
      context.semantics.selectionExpansion.isAwaitingWordSelectionCommit

  private fun resetSelectionExtensionState(context: EditorGestureContext) {
    pendingSelectionExtensionPosition = null
    context.semantics.selectionExpansion.reset()
  }
}

private enum class EditorDoubleTapDragPhase {
  Idle,
  Pending,
  Dragging,
}
