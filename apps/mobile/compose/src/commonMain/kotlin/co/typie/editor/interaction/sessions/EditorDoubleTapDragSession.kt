package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
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
  private var startPosition: Offset? = null
  private var startThresholdPx = 0f
  private var wordSelectionRequest: WordSelectionRequest? = null

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
    wordSelectionRequest = WordSelectionRequest(editor = context.editor)
    context.semantics.selectionExpansion.reset()
    context.semantics.selectionExpansion.awaitWordSelectionApplied()
    context.semantics.contextMenu.hide()
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
    if (!stop()) {
      return false
    }
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    if (wasDragging) {
      if (context.mode.canApply(EditorInteractionEvent.DoubleTapDragEnd)) {
        context.reduceMode(EditorInteractionEvent.DoubleTapDragEnd)
      }
      if (context.semantics.selectionExpansion.isAwaitingWordSelectionApplied) {
        wordSelectionRequest?.showMenuWhenPublished = true
      } else {
        wordSelectionRequest?.let { request ->
          request.showMenuWhenPublished = true
          requestSelectionMenuIfReady(request = request, context = context)
        }
      }
    }
    context.semantics.magnifier.hide()
    return wasActive
  }

  fun captureWordSelectionApplied(
    tap: EditorTapGesture,
    context: EditorGestureContext,
  ): (EditorState?) -> Unit {
    val request = wordSelectionRequest
    return onApplied@{ snapshot ->
      if (
        request == null || wordSelectionRequest !== request || context.editor !== request.editor
      ) {
        return@onApplied
      }
      if (snapshot == null) {
        wordSelectionRequest = null
        resetSelectionExtensionState(context = context)
        return@onApplied
      }
      request.appliedState = snapshot
      context.semantics.selectionExpansion.markWordSelectionApplied()
      if (!tap.hasActivePointer && !active && request.showMenuWhenPublished) {
        requestSelectionMenuIfReady(request = request, context = context)
      } else {
        flushPendingSelectionExtension(request = request, context = context)
        if (!tap.hasActivePointer && !active) {
          wordSelectionRequest = null
          resetSelectionExtensionState(context = context)
        }
      }
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
    wordSelectionRequest = null
    reset()
  }

  fun reset() {
    wordSelectionRequest = null
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
    tap.cancelPendingPresentation(context = context)
    context.reduceMode(EditorInteractionEvent.DoubleTapDragStart)
    if (context.mode != EditorInteractionMode.DoubleTapSelecting) {
      stop()
      context.effects.setScrollGestureLocked(false)
      return false
    }
    tap.clearTapHistory()
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
    val request = wordSelectionRequest ?: return false
    val editor = context.editor
    if (editor !== request.editor) {
      return false
    }
    val selectionContext = context.semantics.selectionExpansion.context(editor)
    if (selectionContext == null) {
      if (context.semantics.selectionExpansion.isAwaitingWordSelectionApplied) {
        request.latestExtension = position
      }
      return false
    }
    if (point.page < 0) {
      return false
    }
    if (editor.dispatchSelectionExtension(point = point, context = selectionContext)) {
      request.latestExtension = position
      context.semantics.magnifier.show(position)
      return true
    }
    return false
  }

  private fun flushPendingSelectionExtension(
    request: WordSelectionRequest,
    context: EditorGestureContext,
  ) {
    val position = request.latestExtension ?: return
    extendSelection(position = position, context = context)
  }

  private fun hasDeferredSelectionExtension(context: EditorGestureContext): Boolean =
    wordSelectionRequest?.latestExtension != null &&
      context.semantics.selectionExpansion.isAwaitingWordSelectionApplied

  private fun resetSelectionExtensionState(context: EditorGestureContext) {
    context.semantics.selectionExpansion.reset()
  }

  private fun requestSelectionMenuIfReady(
    request: WordSelectionRequest,
    context: EditorGestureContext,
  ) {
    if (
      wordSelectionRequest !== request ||
        context.editor !== request.editor ||
        !request.showMenuWhenPublished
    ) {
      return
    }
    val appliedState = request.appliedState ?: return
    if (appliedState.selection.isCollapsed()) {
      wordSelectionRequest = null
      return
    }

    request.showMenuWhenPublished = false
    val extensionPosition = request.latestExtension
    val point = extensionPosition?.let { context.geometry.resolvePoint(positionInNode = it) }
    val extensionContext = point?.let {
      context.semantics.selectionExpansion.context(context.editor)
    }
    if (point == null || extensionContext == null) {
      requestMenuForPublishedSelection(request = request, state = appliedState, context = context)
      return
    }

    context.semantics.pointSelection.launchSelectionExtension(
      editor = request.editor,
      point = point,
      context = extensionContext,
      onApplied = onApplied@{ snapshot ->
          if (wordSelectionRequest !== request) {
            return@onApplied
          }
          requestMenuForPublishedSelection(request = request, state = snapshot, context = context)
        },
      afterDispatch = { dispatched ->
        if (wordSelectionRequest === request && !dispatched) {
          requestMenuForPublishedSelection(
            request = request,
            state = appliedState,
            context = context,
          )
        }
      },
    )
  }

  private fun requestMenuForPublishedSelection(
    request: WordSelectionRequest,
    state: EditorState,
    context: EditorGestureContext,
  ) {
    if (wordSelectionRequest !== request || context.editor !== request.editor) {
      return
    }
    wordSelectionRequest = null
    if (state.selection.isCollapsed()) {
      return
    }
    context.semantics.contextMenu.requestShowForAppliedSelection(
      editor = request.editor,
      state = state,
    )
  }

  private class WordSelectionRequest(val editor: Editor) {
    var appliedState: EditorState? = null
    var latestExtension: Offset? = null
    var showMenuWhenPublished = false
  }
}

private enum class EditorDoubleTapDragPhase {
  Idle,
  Pending,
  Dragging,
}
