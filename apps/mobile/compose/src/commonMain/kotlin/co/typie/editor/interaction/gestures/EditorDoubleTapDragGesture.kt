package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionCommand
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.semantics.dispatchSelectionExtension

private const val EditorDoubleTapDragStartThresholdPx = 4f

internal class EditorDoubleTapDragGesture {
  private var phase = EditorDoubleTapDragPhase.Idle
  private var pendingSelectionExtensionPosition: Offset? = null
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
    pendingSelectionExtensionPosition = null
    stop()
  }

  fun prepareForDrag(
    position: Offset,
    tap: EditorTapGesture,
    context: EditorGestureContext,
  ): Boolean {
    if (!context.can(EditorInteractionCommand.DoubleTapPrepareDrag)) {
      return false
    }
    context.cancelTapDispatch()
    tap.markTapDispatched()
    context.semantics.selectionExpansion.reset()
    context.semantics.selectionExpansion.awaitWordSelectionCommit()
    prepare(startPosition = position)
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
    if (wasDragging) {
      context.transition(EditorInteractionEvent.DoubleTapDragEnd)
    } else if (wasPending) {
      // TODO(editor-parity): legacy shows the selection context menu when a double tap selects a
      // range but never crosses the drag threshold. KMP does not host that menu state yet.
    }
    return wasActive
  }

  fun onWordSelectionCommitted(tap: EditorTapGesture, context: EditorGestureContext) {
    context.semantics.selectionExpansion.markWordSelectionCommitted()
    flushPendingSelectionExtension(context = context)
    if (!tap.hasActivePointer && !active) {
      resetSelectionExtensionState(context = context)
    }
  }

  fun cleanupAfterPointerUp(tap: EditorTapGesture, context: EditorGestureContext) {
    if (!tap.hasActivePointer && !hasDeferredSelectionExtension(context = context)) {
      resetSelectionExtensionState(context = context)
    }
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    resetSelectionExtensionState(context = context)
    reset()
  }

  private fun start(tap: EditorTapGesture, context: EditorGestureContext): Boolean {
    if (!context.can(EditorInteractionCommand.DoubleTapStartDrag)) {
      return false
    }
    context.cancelTapDispatch()
    tap.markTapDispatched()
    if (!begin()) {
      return false
    }
    if (!context.can(EditorInteractionCommand.DoubleTapBeginSelecting)) {
      stop()
      return false
    }
    context.transition(EditorInteractionEvent.DoubleTapDragStart)
    return true
  }

  private fun updateSelection(position: Offset, context: EditorGestureContext): Boolean {
    if (
      !context.can(
        EditorInteractionCommand.DoubleTapUpdateSelection(
          localPosition = position,
          dragStartPosition = startPosition,
        )
      )
    ) {
      return false
    }

    return extendSelection(position = position, context = context)
  }

  private fun extendSelection(position: Offset, context: EditorGestureContext): Boolean {
    val point = context.resolvePoint(positionInNode = position) ?: return false
    val editor = context.editor
    val selectionContext = context.semantics.selectionExpansion.context(editor)
    if (selectionContext == null) {
      if (context.semantics.selectionExpansion.isAwaitingWordSelectionCommit) {
        pendingSelectionExtensionPosition = position
      }
      return false
    }
    if (
      !context.can(
        EditorInteractionCommand.DoubleTapExtendSelection(
          page = point.page,
          hasSelectionContext = true,
        )
      )
    ) {
      return false
    }
    if (editor.dispatchSelectionExtension(point = point, context = selectionContext)) {
      pendingSelectionExtensionPosition = null
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
