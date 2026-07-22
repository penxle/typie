package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.EditorTableCellSelectionHandleTouchTargetDp
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.contains
import co.typie.editor.interaction.resolveActiveTableCellSelection
import co.typie.editor.interaction.semantics.dispatchSelectionHandleExtension
import co.typie.editor.interaction.sessions.EditorTableHandleDragContext
import co.typie.editor.interaction.sessions.EditorTableHandleDragSession

internal class EditorTableHandleGesture(
  private val contextProvider: () -> EditorGestureContext,
  private val onHandoffToSelectionHandle:
    (EditorTableHandleDragUpdate.HandoffToSelectionHandle) -> Boolean =
    {
      false
    },
  private val session: EditorTableHandleDragSession = EditorTableHandleDragSession(),
) {
  private var dragSlop = 0f

  val pendingDrag: Boolean
    get() = session.pendingDrag

  val activeDrag: Boolean
    get() = session.activeDrag

  val dragging: Boolean
    get() = session.dragging

  fun updateDragSlop(dragSlop: Float) {
    this.dragSlop = dragSlop.coerceAtLeast(0f)
  }

  fun shouldStartDrag(position: Offset): Boolean =
    session.hasMovedPastSlop(touchPosition = position, dragSlop = dragSlop)

  fun hitTest(position: Offset): Boolean {
    val context = contextProvider()
    val density = context.geometry.density
    if (density <= 0f) {
      return false
    }
    val handleCenter = activeHandleCenter(context = context) ?: return false
    val halfTouchTarget = EditorTableCellSelectionHandleTouchTargetDp * density / 2f
    return position.x >= handleCenter.x - halfTouchTarget &&
      position.x <= handleCenter.x + halfTouchTarget &&
      position.y >= handleCenter.y - halfTouchTarget &&
      position.y <= handleCenter.y + halfTouchTarget
  }

  fun handleDragDown(position: Offset): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.TableHandleDragStart)) {
      return false
    }
    if (!hitTest(position)) {
      return false
    }
    if (!session.beginPending(touchPosition = position)) {
      return false
    }
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    context.semantics.contextMenu.hide()
    return true
  }

  fun handleDragStart(position: Offset): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.TableHandleDragStart)) {
      resetPointerOwnedState(context = context)
      return false
    }
    val activeSelection =
      resolveActiveTableCellSelection(context.editor)
        ?: run {
          resetPointerOwnedState(context = context)
          return false
        }
    val handleCenter =
      activeHandleCenter(context = context)
        ?: run {
          resetPointerOwnedState(context = context)
          return false
        }
    val baseSelection =
      context.editor.selection
        ?: run {
          resetPointerOwnedState(context = context)
          return false
        }
    if (
      !session.beginDrag(
        touchPosition = position,
        handleCenter = handleCenter,
        tableId = activeSelection.overlay.tableId,
        anchor = baseSelection.anchor,
        baseSelection = baseSelection,
      )
    ) {
      resetPointerOwnedState(context = context)
      return false
    }

    context.semantics.contextMenu.hide()
    context.semantics.magnifier.hide()
    context.effects.cancelTapDispatch()
    context.effects.cancelLongPressDispatch()
    context.effects.setScrollGestureLocked(true)
    context.reduceMode(EditorInteractionEvent.TableHandleDragStart)
    if (context.mode != EditorInteractionMode.TableCellHandleDragging) {
      resetPointerOwnedState(context = context)
      return false
    }
    return true
  }

  fun adoptSelectionHandleDrag(
    touchPosition: Offset,
    handlePosition: Offset,
    tableId: String,
    anchor: Position,
    baseSelection: Selection,
  ): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.TableHandleDragStart)) {
      resetPointerOwnedState(context = context)
      return false
    }
    if (
      !session.adoptDrag(
        touchPosition = touchPosition,
        handleCenter = handlePosition,
        tableId = tableId,
        anchor = anchor,
        baseSelection = baseSelection,
      )
    ) {
      resetPointerOwnedState(context = context)
      return false
    }

    context.semantics.contextMenu.hide()
    context.semantics.magnifier.hide()
    context.effects.setScrollGestureLocked(true)
    context.reduceMode(EditorInteractionEvent.TableHandleDragStart)
    if (context.mode != EditorInteractionMode.TableCellHandleDragging) {
      resetPointerOwnedState(context = context)
      return false
    }
    return true
  }

  fun handleDragUpdate(position: Offset): EditorTableHandleDragUpdate {
    val context = contextProvider()
    if (context.mode != EditorInteractionMode.TableCellHandleDragging) {
      return EditorTableHandleDragUpdate.NotConsumed
    }
    val drag = session.activeContext ?: return EditorTableHandleDragUpdate.NotConsumed
    val selectionPosition =
      session.selectionPosition(touchPosition = position)
        ?: return EditorTableHandleDragUpdate.NotConsumed

    context.semantics.magnifier.show(selectionPosition)
    context.semantics.edgeAutoScroll.trackSelectionHandle(
      edgePosition = position,
      dispatchPosition = selectionPosition,
      context = context,
      dispatch = { scrolled ->
        when (
          val update =
            handleDragPoint(
              context = context,
              touchPosition = scrolled.edgePosition,
              handlePosition = scrolled.dispatchPosition,
              point = scrolled.point,
              drag = drag,
            )
        ) {
          EditorTableHandleDragUpdate.NotConsumed -> false
          EditorTableHandleDragUpdate.Consumed -> true
          is EditorTableHandleDragUpdate.HandoffToSelectionHandle ->
            onHandoffToSelectionHandle(update)
        }
      },
    )

    val point =
      context.geometry.resolvePoint(positionInNode = selectionPosition)
        ?: return EditorTableHandleDragUpdate.Consumed
    return handleDragPoint(
      context = context,
      touchPosition = position,
      handlePosition = selectionPosition,
      point = point,
      drag = drag,
    )
  }

  fun handleDragEnd(): Boolean {
    val context = contextProvider()
    val wasActive = dragging
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
    if (context.mode.canApply(EditorInteractionEvent.TableHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.TableHandleDragEnd)
    }
    return wasActive
  }

  fun cancel() {
    val context = contextProvider()
    resetPointerOwnedState(context = context)
    if (context.mode.canApply(EditorInteractionEvent.TableHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.TableHandleDragEnd)
    }
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
  }

  fun reset() = session.reset()

  private fun activeHandleCenter(context: EditorGestureContext): Offset? {
    val activeSelection = resolveActiveTableCellSelection(context.editor) ?: return null
    val center = activeSelection.geometry.handleCenter ?: return null
    return context.geometry.resolvePagePosition(
      page = activeSelection.overlay.pageIdx,
      x = center.x,
      y = center.y,
    )
  }

  private fun handleDragPoint(
    context: EditorGestureContext,
    touchPosition: Offset,
    handlePosition: Offset,
    point: PagePoint,
    drag: EditorTableHandleDragContext,
  ): EditorTableHandleDragUpdate {
    if (point.page < 0) {
      return EditorTableHandleDragUpdate.Consumed
    }

    if (
      !context.editor.tableOverlays.any { overlay ->
        overlay.tableId == drag.tableId &&
          overlay.contains(
            point = point,
            layoutMode = context.editor.rootAttrs?.layoutMode,
            project = context.geometry::resolvePagePosition,
          )
      }
    ) {
      return EditorTableHandleDragUpdate.HandoffToSelectionHandle(
        touchPosition = touchPosition,
        handlePosition = handlePosition,
        tableId = drag.tableId,
        anchor = drag.anchor,
        baseSelection = drag.baseSelection,
      )
    }

    context.editor.dispatchSelectionHandleExtension(
      point = point,
      anchor = drag.anchor,
      baseSelection = drag.baseSelection,
    )
    return EditorTableHandleDragUpdate.Consumed
  }
}

internal sealed interface EditorTableHandleDragUpdate {
  data object NotConsumed : EditorTableHandleDragUpdate

  data object Consumed : EditorTableHandleDragUpdate

  data class HandoffToSelectionHandle(
    val touchPosition: Offset,
    val handlePosition: Offset,
    val tableId: String,
    val anchor: Position,
    val baseSelection: Selection,
  ) : EditorTableHandleDragUpdate
}
