package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.editor.interaction.hasActiveTableCellSelection
import co.typie.editor.interaction.sessions.EditorSelectionHandleDragSession
import kotlin.math.max

internal enum class EditorSelectionHandleType {
  From,
  To,
}

internal data class EditorSelectionHandleTableCellHandoff(
  val touchPosition: Offset,
  val handlePosition: Offset,
  val tableId: String,
  val anchor: Position,
  val baseSelection: Selection,
)

internal const val EditorSelectionHandleRadiusDp = 8f
internal const val EditorSelectionHandleStemWidthDp = 2f
internal const val EditorSelectionHandleTouchTargetDp = 44f

internal data class EditorSelectionHandleGeometry(
  val touchTargetTopLeft: Offset,
  val touchTargetSize: Size,
  val paintTopLeftInTouchTarget: Offset,
  val stemHeightPx: Float,
  val radiusPx: Float,
  val stemWidthPx: Float,
) {
  fun containsTouch(position: Offset): Boolean =
    position.x >= touchTargetTopLeft.x &&
      position.x <= touchTargetTopLeft.x + touchTargetSize.width &&
      position.y >= touchTargetTopLeft.y &&
      position.y <= touchTargetTopLeft.y + touchTargetSize.height
}

internal fun resolveSelectionHandleGeometry(
  type: EditorSelectionHandleType,
  endpointTopLeftInOverlay: Offset,
  stemHeightPx: Float,
  radiusPx: Float,
  stemWidthPx: Float,
  touchTargetPx: Float,
): EditorSelectionHandleGeometry {
  val totalHeightPx = radiusPx * 2f + stemHeightPx
  val effectiveTouchHeightPx = max(totalHeightPx, touchTargetPx)
  val customPaintTop = if (type == EditorSelectionHandleType.From) -radiusPx * 2f else 0f
  val handleCenterY = customPaintTop + totalHeightPx / 2f
  val touchTargetTop = handleCenterY - effectiveTouchHeightPx / 2f
  val handleXOffset =
    if (type == EditorSelectionHandleType.From) {
      -stemWidthPx / 2f
    } else {
      stemWidthPx / 2f
    }
  val touchTargetLeft = handleXOffset - touchTargetPx / 2f

  return EditorSelectionHandleGeometry(
    touchTargetTopLeft = endpointTopLeftInOverlay + Offset(touchTargetLeft, touchTargetTop),
    touchTargetSize = Size(width = touchTargetPx, height = effectiveTouchHeightPx),
    paintTopLeftInTouchTarget =
      Offset(x = (touchTargetPx - radiusPx * 2f) / 2f, y = customPaintTop - touchTargetTop),
    stemHeightPx = stemHeightPx,
    radiusPx = radiusPx,
    stemWidthPx = stemWidthPx,
  )
}

internal class EditorSelectionHandleGesture(
  private val contextProvider: () -> EditorGestureContext,
  private val session: EditorSelectionHandleDragSession = EditorSelectionHandleDragSession(),
) {
  val pendingDrag: Boolean
    get() = session.pendingDrag

  val pendingType: EditorSelectionHandleType?
    get() = session.pendingType

  val activeDrag: Boolean
    get() = session.activeDrag

  val activeType: EditorSelectionHandleType?
    get() = session.activeType

  fun hitTest(position: Offset): EditorSelectionHandleType? {
    val context = contextProvider()
    val density = context.geometry.density
    if (density <= 0f) {
      return null
    }
    if (context.editor.selection.isCollapsed()) {
      return null
    }
    if (hasActiveTableCellSelection(context.editor)) {
      return null
    }
    val radiusPx = EditorSelectionHandleRadiusDp * density
    val stemWidthPx = EditorSelectionHandleStemWidthDp * density
    val touchTargetPx = EditorSelectionHandleTouchTargetDp * density
    val endpoints = context.editor.selectionEndpoints() ?: return null
    return listOf(
        EditorSelectionHandleType.To to endpoints.to,
        EditorSelectionHandleType.From to endpoints.from,
      )
      .firstOrNull { (type, endpoint) ->
        hitTestEndpoint(
          type = type,
          endpoint = endpoint,
          position = position,
          context = context,
          radiusPx = radiusPx,
          stemWidthPx = stemWidthPx,
          touchTargetPx = touchTargetPx,
        )
      }
      ?.first
  }

  private fun hitTestEndpoint(
    type: EditorSelectionHandleType,
    endpoint: PageRect,
    position: Offset,
    context: EditorGestureContext,
    radiusPx: Float,
    stemWidthPx: Float,
    touchTargetPx: Float,
  ): Boolean {
    val rect = endpoint.rect
    val topLeft =
      context.geometry.resolvePagePosition(page = endpoint.pageIdx, x = rect.x, y = rect.y)
        ?: return false
    val bottom =
      context.geometry.resolvePagePosition(
        page = endpoint.pageIdx,
        x = rect.x,
        y = rect.y + rect.height,
      ) ?: return false
    val geometry =
      resolveSelectionHandleGeometry(
        type = type,
        endpointTopLeftInOverlay = topLeft,
        stemHeightPx = (bottom.y - topLeft.y).coerceAtLeast(0f),
        radiusPx = radiusPx,
        stemWidthPx = stemWidthPx,
        touchTargetPx = touchTargetPx,
      )
    return geometry.containsTouch(position)
  }

  fun handleDragDown(
    type: EditorSelectionHandleType,
    position: Offset,
    preserveTapDispatch: Boolean = false,
  ): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      return false
    }
    if (!session.beginPending(type = type, touchPosition = position)) {
      return false
    }
    if (!preserveTapDispatch) {
      context.effects.cancelTapDispatch()
    }
    context.effects.cancelLongPressDispatch()
    context.effects.setScrollGestureLocked(true)
    context.uiState.contextMenu.hide()
    return true
  }

  fun handleDragStart(type: EditorSelectionHandleType, position: Offset): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      resetPointerOwnedState(context = context)
      return false
    }
    if (!session.beginDrag(type = type, touchPosition = position, context = context)) {
      resetPointerOwnedState(context = context)
      return false
    }

    context.uiState.contextMenu.hide()
    context.semantics.magnifier.hide()
    context.reduceMode(EditorInteractionEvent.SelectionHandleDragStart)
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      resetPointerOwnedState(context = context)
      return false
    }
    return true
  }

  fun adoptTableCellDrag(
    touchPosition: Offset,
    handlePosition: Offset,
    tableId: String,
    anchor: Position,
    baseSelection: Selection,
  ): Boolean {
    val context = contextProvider()
    if (!context.mode.canApply(EditorInteractionEvent.SelectionHandleDragStart)) {
      resetPointerOwnedState(context = context)
      return false
    }
    if (
      !session.adoptDrag(
        type = EditorSelectionHandleType.To,
        touchPosition = touchPosition,
        handlePosition = handlePosition,
        tableId = tableId,
        anchor = anchor,
        baseSelection = baseSelection,
      )
    ) {
      resetPointerOwnedState(context = context)
      return false
    }

    context.uiState.contextMenu.hide()
    context.semantics.magnifier.hide()
    context.effects.setScrollGestureLocked(true)
    context.reduceMode(EditorInteractionEvent.SelectionHandleDragStart)
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      resetPointerOwnedState(context = context)
      return false
    }
    session.update(
      type = EditorSelectionHandleType.To,
      touchPosition = touchPosition,
      context = context,
    )
    return true
  }

  fun tableCellHandoff(
    type: EditorSelectionHandleType,
    position: Offset,
  ): EditorSelectionHandleTableCellHandoff? =
    session.tableCellHandoff(type = type, touchPosition = position, context = contextProvider())

  fun handleDragUpdate(type: EditorSelectionHandleType, position: Offset): Boolean {
    val context = contextProvider()
    if (context.mode != EditorInteractionMode.SelectionHandleDragging) {
      return false
    }
    if (!session.isActiveType(type)) {
      return false
    }
    session.update(type = type, touchPosition = position, context = context)
    return true
  }

  fun handleDragHandoff(): Boolean {
    val context = contextProvider()
    val wasActive = activeDrag
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
    if (context.mode.canApply(EditorInteractionEvent.SelectionHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.SelectionHandleDragEnd)
    }
    return wasActive
  }

  fun handleDragEnd(type: EditorSelectionHandleType): Boolean {
    val context = contextProvider()
    if (activeDrag && !session.isActiveType(type)) {
      return false
    }
    val wasActive = activeDrag || pendingDrag
    val wasDragging = activeDrag
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
    if (context.mode.canApply(EditorInteractionEvent.SelectionHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.SelectionHandleDragEnd)
    }
    if (wasDragging && !context.editor.selection.isCollapsed()) {
      context.uiState.contextMenu.show(context.editor.state)
      context.uiState.contextMenu.requestShowAfterSelectionCommit()
    }
    return wasActive
  }

  fun cancel() {
    val context = contextProvider()
    resetPointerOwnedState(context = context)
    if (context.mode.canApply(EditorInteractionEvent.SelectionHandleDragEnd)) {
      context.reduceMode(EditorInteractionEvent.SelectionHandleDragEnd)
    }
  }

  fun resetPointerOwnedState(context: EditorGestureContext) {
    session.reset()
    context.semantics.edgeAutoScroll.stop()
    context.effects.setScrollGestureLocked(false)
    context.semantics.magnifier.hide()
  }

  fun reset() = session.reset()
}
