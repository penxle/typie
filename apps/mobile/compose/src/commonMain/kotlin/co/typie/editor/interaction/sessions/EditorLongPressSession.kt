package co.typie.editor.interaction.sessions

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.isLongPressing
import co.typie.editor.interaction.isViewportZooming
import co.typie.editor.interaction.semantics.dispatchSelectionExtension
import co.typie.editor.interaction.semantics.enqueuePrimaryClick

internal enum class EditorLongPressSemanticIntent {
  CursorMove,
  WordSelection,
}

internal class EditorLongPressSession {
  private var activePointerId: Long? = null
  private var semanticIntent = EditorLongPressSemanticIntent.CursorMove

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
    if (!begin(pointerId = pointerId, semanticIntent = semanticIntent)) {
      return false
    }
    context.effects.setScrollGestureLocked(true)

    context.uiState.contextMenu.hide()
    context.semantics.magnifier.show(position)
    if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
      context.semantics.selectionExpansion.awaitWordSelectionCommit(
        baselineSelection = context.editor.state.selection
      )
    }

    context.reduceMode(event)
    if (context.mode != expectedMode) {
      end()
      context.effects.setScrollGestureLocked(false)
      context.semantics.magnifier.hide()
      context.semantics.selectionExpansion.reset()
      return false
    }

    if (semanticIntent == EditorLongPressSemanticIntent.WordSelection) {
      dispatchWordSelectionAt(point = point, context = context)
    }
    return true
  }

  fun update(position: Offset, context: EditorGestureContext): Boolean {
    if (context.mode.isViewportZooming || !context.mode.isLongPressing) {
      return false
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
      )
    }
    val point = context.geometry.resolvePoint(positionInNode = position) ?: return true
    if (point.page < 0) {
      return true
    }

    if (isWordSelection) {
      val selectionContext =
        context.semantics.selectionExpansion.context(context.editor) ?: return true
      return context.editor.dispatchSelectionExtension(point = point, context = selectionContext)
    }

    return context.semantics.cursorMove.enqueuePrimaryClick(
      editor = context.editor,
      point = point,
      clickCount = 1,
    )
  }

  fun finish(context: EditorGestureContext): Boolean {
    val event =
      if (isWordSelection) {
        EditorInteractionEvent.LongPressWordEnd
      } else {
        EditorInteractionEvent.LongPressEnd
      }
    if (!context.mode.canApply(event)) {
      end()
      context.semantics.edgeAutoScroll.stop()
      context.effects.setScrollGestureLocked(false)
      context.semantics.magnifier.hide()
      context.semantics.selectionExpansion.reset()
      return false
    }

    val endedWord = isWordSelection
    if (endedWord && !context.editor.selection.isCollapsed()) {
      context.uiState.contextMenu.show(context.editor.state)
    } else if (endedWord) {
      context.uiState.contextMenu.requestShowAfterSelectionCommit()
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
    semanticIntent = EditorLongPressSemanticIntent.CursorMove
  }

  fun reset() {
    end()
  }

  private fun begin(pointerId: Long, semanticIntent: EditorLongPressSemanticIntent): Boolean {
    if (active) {
      return false
    }
    activePointerId = pointerId
    this.semanticIntent = semanticIntent
    return true
  }

  private fun dispatchWordSelectionAt(point: PagePoint, context: EditorGestureContext) {
    context.semantics.cursorMove.launchPrimaryClick(
      editor = context.editor,
      point = point,
      clickCount = 2,
      afterDispatch = {
        context.semantics.selectionExpansion.markWordSelectionCommitted()
        context.uiState.contextMenu.showAfterSelectionCommitIfRequested(context.editor.state)
      },
    )
  }
}
