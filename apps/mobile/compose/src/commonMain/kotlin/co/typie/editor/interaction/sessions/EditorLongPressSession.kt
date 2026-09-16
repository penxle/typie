package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.isLongPressing
import co.typie.editor.interaction.isViewportZooming
import co.typie.editor.interaction.semantics.EditorLongPressSemanticIntent
import co.typie.editor.interaction.semantics.dispatchSelectionExtension
import co.typie.platform.Platform
import kotlin.math.abs

internal class EditorLongPressSession {
  private var activePointerId: Long? = null
  private var semanticIntent = EditorLongPressSemanticIntent.CursorMove
  private var wordSelectionRequest: WordSelectionRequest? = null
  private var lastCursorPoint: PagePoint? = null
  private var dragOrigin = Offset.Zero
  private var dragState = DragState.Pending

  val active: Boolean
    get() = activePointerId != null

  val isWordSelection: Boolean
    get() = semanticIntent == EditorLongPressSemanticIntent.WordSelection

  fun isActivePointer(pointerId: Long): Boolean = activePointerId == pointerId

  fun start(
    pointerId: Long,
    position: Offset,
    point: PagePoint,
    semanticIntent: EditorLongPressSemanticIntent,
    context: EditorGestureContext,
  ): Boolean {
    val event =
      if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
        EditorInteractionEvent.LongPressWordStart
      } else {
        EditorInteractionEvent.LongPressStart
      }
    val expectedMode =
      if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
        EditorInteractionMode.LongPressWordSelecting
      } else {
        EditorInteractionMode.LongPressSelecting
      }

    if (!context.mode.canApply(event)) {
      return false
    }

    context.effects.cancelTapDispatch()
    if (!begin(pointerId = pointerId, semanticIntent = semanticIntent, editor = context.editor)) {
      return false
    }
    context.effects.setScrollGestureLocked(true)

    context.semantics.contextMenu.hide()
    if (context.platform != Platform.Android) context.semantics.magnifier.show(position)
    if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
      context.semantics.selectionExpansion.awaitWordSelectionApplied(
        baselineSelection = context.editor.appliedState.selection
      )
    }

    context.reduceMode(event)
    if (context.mode != expectedMode) {
      reset()
      context.effects.setScrollGestureLocked(false)
      context.semantics.magnifier.hide()
      context.semantics.selectionExpansion.reset()
      return false
    }

    dragOrigin = position
    if (semanticIntent == EditorLongPressSemanticIntent.ContextMenu) {
      context.semantics.contextMenu.requestShowForAppliedSelection(
        context.editor,
        context.editor.publishedState,
      )
    } else if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
      wordSelectionRequest?.showMenuWhenPublished = context.platform == Platform.Android
      dispatchWordSelectionAt(point = point, context = context)
    } else if (context.platform == Platform.Android) {
      val state =
        context.semantics.pointSelection.applySelection(
          context.editor,
          SelectionOp.SetAt(point.page, point.x, point.y),
        )
      if (state != null) {
        context.cursorHandle.show(state, context.effects)
        context.semantics.contextMenu.requestShowForAppliedSelection(
          context.editor,
          state,
          allowCollapsed = true,
        )
      }
    }
    return true
  }

  fun update(position: Offset, dragSlop: Float, context: EditorGestureContext): Boolean {
    if (context.mode.isViewportZooming || !context.mode.isLongPressing) {
      return false
    }
    if (semanticIntent == EditorLongPressSemanticIntent.ContextMenu) return true
    if (context.platform == Platform.Android) {
      when (dragState) {
        DragState.Pending -> {
          val delta = position - dragOrigin
          if (delta.getDistance() <= dragSlop) return true
          // EditText fixes the initial direction when touch slop is crossed. A swipe within
          // 45 degrees of vertical does not start cursor dragging, even if it turns later.
          if (!isWordSelection && abs(delta.x) <= abs(delta.y)) {
            dragState = DragState.Rejected
            return true
          }
          dragState = DragState.Dragging
          wordSelectionRequest?.showMenuWhenPublished = false
          context.semantics.contextMenu.hide()
          if (!isWordSelection) {
            context.cursorHandle.show(context.editor.appliedState, context.effects)
            context.cursorHandle.hold()
          }
        }
        DragState.Rejected -> return true
        DragState.Dragging -> Unit
      }
    }
    context.semantics.magnifier.show(position)
    if (isWordSelection) {
      context.semantics.edgeAutoScroll.trackSelectionExpansion(
        edgePosition = position,
        dispatchPosition = position,
        context = context,
      )
    } else {
      context.semantics.edgeAutoScroll.trackCursorMove(
        edgePosition = position,
        dispatchPosition = position,
        context = context,
        dispatch = { point -> dispatchCursorMove(point, context) },
      )
    }
    val point = context.geometry.resolvePoint(positionInNode = position) ?: return true
    if (point.page < 0) {
      return true
    }

    if (isWordSelection) {
      val request = wordSelectionRequest
      if (request?.editor !== context.editor) {
        return false
      }
      if (context.semantics.selectionExpansion.isAwaitingWordSelectionApplied) {
        request.latestExtension = position
      }
      val selectionContext =
        context.semantics.selectionExpansion.context(context.editor) ?: return true
      request.appliedState = context.editor.appliedState
      val dispatched =
        context.editor.dispatchSelectionExtension(point = point, context = selectionContext)
      if (dispatched) {
        request.latestExtension = position
      }
      return dispatched
    }

    return dispatchCursorMove(point, context)
  }

  private fun dispatchCursorMove(point: PagePoint, context: EditorGestureContext): Boolean {
    val dispatched = context.semantics.pointSelection.enqueueCursorMove(context.editor, point)
    if (dispatched) lastCursorPoint = point
    return dispatched
  }

  fun finish(context: EditorGestureContext): Boolean {
    val event =
      if (isWordSelection) {
        EditorInteractionEvent.LongPressWordEnd
      } else {
        EditorInteractionEvent.LongPressEnd
      }
    if (!context.mode.canApply(event)) {
      reset()
      context.semantics.edgeAutoScroll.stop()
      context.effects.setScrollGestureLocked(false)
      context.semantics.magnifier.hide()
      context.semantics.selectionExpansion.reset()
      return false
    }

    if (semanticIntent == EditorLongPressSemanticIntent.ContextMenu) {
      context.semantics.contextMenu.requestShowForAppliedSelection(
        context.editor,
        context.editor.publishedState,
      )
    } else if (isWordSelection) {
      wordSelectionRequest?.let { request ->
        request.showMenuWhenPublished = true
        requestWordSelectionMenuIfReady(request = request, context = context)
      }
    } else if (context.platform == Platform.Android && context.editing && !context.readOnly) {
      val point = lastCursorPoint
      val state =
        if (point != null) {
          context.semantics.pointSelection.applySelection(
            context.editor,
            SelectionOp.SetAt(point.page, point.x, point.y),
          )
        } else context.editor.appliedState
      if (state != null) {
        context.cursorHandle.show(state, context.effects)
        context.semantics.contextMenu.requestShowForAppliedSelection(
          context.editor,
          state,
          allowCollapsed = true,
        )
      } else {
        context.cursorHandle.hide()
      }
    }
    context.reduceMode(event)
    end()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
    context.semantics.selectionExpansion.reset()
    return true
  }

  fun end() {
    activePointerId = null
    lastCursorPoint = null
    dragOrigin = Offset.Zero
    dragState = DragState.Pending
    semanticIntent = EditorLongPressSemanticIntent.CursorMove
  }

  fun reset() {
    wordSelectionRequest = null
    end()
  }

  private fun begin(
    pointerId: Long,
    semanticIntent: EditorLongPressSemanticIntent,
    editor: Editor,
  ): Boolean {
    if (active) {
      return false
    }
    activePointerId = pointerId
    this.semanticIntent = semanticIntent
    wordSelectionRequest =
      if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
        WordSelectionRequest(editor = editor)
      } else {
        null
      }
    return true
  }

  private fun dispatchWordSelectionAt(point: PagePoint, context: EditorGestureContext) {
    val request = wordSelectionRequest ?: return
    context.semantics.pointSelection.launchUnitSelection(
      editor = request.editor,
      point = point,
      unit = SelectionPointUnit.Word,
      onApplied = { snapshot ->
        if (wordSelectionRequest === request && context.editor === request.editor) {
          request.appliedState = snapshot
        }
      },
      afterDispatch = afterDispatch@{ dispatched ->
          if (wordSelectionRequest !== request || context.editor !== request.editor) {
            return@afterDispatch
          }
          if (!dispatched) {
            wordSelectionRequest = null
            context.semantics.selectionExpansion.reset()
            return@afterDispatch
          }
          context.semantics.selectionExpansion.markWordSelectionApplied()
          requestWordSelectionMenuIfReady(request = request, context = context)
        },
    )
  }

  private fun requestWordSelectionMenuIfReady(
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
      requestMenuForPublishedSelection(request, appliedState, context)
      return
    }
    val extensionPoint =
      request.latestExtension?.let { context.geometry.resolvePoint(positionInNode = it) }
    val extensionContext = extensionPoint?.let {
      context.semantics.selectionExpansion.context(context.editor)
    }
    if (extensionPoint == null || extensionContext == null) {
      requestMenuForPublishedSelection(request = request, state = appliedState, context = context)
      return
    }

    context.semantics.pointSelection.launchSelectionExtension(
      editor = request.editor,
      point = extensionPoint,
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
    val allowCollapsed =
      context.platform == Platform.Android && context.editing && !context.readOnly
    if (state.selection.isCollapsed() && !allowCollapsed) {
      return
    }
    if (allowCollapsed && state.selection.isCollapsed())
      context.cursorHandle.show(state, context.effects)
    context.semantics.contextMenu.requestShowForAppliedSelection(
      editor = request.editor,
      state = state,
      allowCollapsed = allowCollapsed,
    )
  }

  private enum class DragState {
    Pending,
    Dragging,
    Rejected,
  }

  private class WordSelectionRequest(val editor: Editor) {
    var appliedState: EditorState? = null
    var latestExtension: Offset? = null
    var showMenuWhenPublished = false
  }
}
