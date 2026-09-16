package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.PagePoint
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.canApply
import co.typie.platform.Platform
import co.typie.ui.input.isDirectTouchInteraction
import kotlin.math.abs
import kotlin.math.max

internal class EditorCursorDragGesture {
  private var downPosition: Offset? = null
  private var grabOffset = Offset.Zero
  private var lastCursorPoint: PagePoint? = null
  private var source = Source.Caret
  private var menuWasVisible = false
  private var handleTapEnabled = false
  var active = false
    private set

  val pending: Boolean
    get() = downPosition != null && !active

  val pendingHandle: Boolean
    get() = pending && source == Source.Handle

  fun prepareHandle(position: Offset, context: EditorGestureContext, tapEnabled: Boolean): Boolean {
    if (
      context.platform != Platform.Android ||
        !context.cursorHandle.visible ||
        !context.pointerType.isDirectTouchInteraction() ||
        !context.isFocused ||
        !context.editing ||
        context.readOnly ||
        !context.mode.canApply(EditorInteractionEvent.CursorDragStart)
    )
      return false
    val state = context.editor.publishedState
    if (!context.cursorHandle.isVisibleFor(state)) return false
    if (state.selection == null || !state.selection.isCollapsed()) return false
    val cursor = state.cursor ?: return false
    val top =
      context.geometry.resolvePagePosition(cursor.pageIdx, cursor.caret.x, cursor.caret.y)
        ?: return false
    val bottom =
      context.geometry.resolvePagePosition(
        cursor.pageIdx,
        cursor.caret.x,
        cursor.caret.y + cursor.caret.height,
      ) ?: return false
    val density = context.geometry.density
    val geometry =
      resolveSelectionHandleGeometry(
        type = EditorSelectionHandleType.Cursor,
        endpointTopLeftInOverlay = top,
        stemHeightPx = (bottom.y - top.y).coerceAtLeast(0f),
        radiusPx = EditorAndroidCursorHandleRadiusDp * density,
        stemWidthPx = 0f,
        touchTargetPx = EditorSelectionHandleTouchTargetDp * density,
        platform = Platform.Android,
        image = context.selectionHandleImages[EditorSelectionHandleType.Cursor],
      )
    if (!geometry.containsTouch(position)) return false
    downPosition = position
    grabOffset = (top + bottom) / 2f - position
    source = Source.Handle
    menuWasVisible = context.semantics.contextMenu.visible
    handleTapEnabled = tapEnabled
    context.cursorHandle.hold()
    context.effects.setScrollGestureLocked(true)
    context.semantics.contextMenu.hide()
    return true
  }

  fun prepare(position: Offset, context: EditorGestureContext) {
    if (
      (context.platform != Platform.iOS && context.platform != Platform.Android) ||
        !context.pointerType.isDirectTouchInteraction() ||
        !context.isFocused ||
        !context.editing ||
        context.readOnly ||
        !context.mode.canApply(EditorInteractionEvent.CursorDragStart)
    )
      return
    val state = context.editor.publishedState
    if (state.selection == null || !state.selection.isCollapsed()) return
    if (context.platform == Platform.Android) {
      downPosition = position
      source = Source.Body
      return
    }
    val cursor = state.cursor ?: return
    val caret = cursor.caret
    val top = context.geometry.resolvePagePosition(cursor.pageIdx, caret.x, caret.y) ?: return
    val bottom =
      context.geometry.resolvePagePosition(cursor.pageIdx, caret.x, caret.y + caret.height)
        ?: return
    val padding = EditorSelectionHandleTouchTargetDp * context.geometry.density / 2f
    val center = (top + bottom) / 2f
    if (
      abs(position.x - center.x) > padding ||
        abs(position.y - center.y) > max(padding, (bottom.y - top.y) / 2f)
    )
      return
    downPosition = position
    grabOffset = center - position
  }

  fun shouldStart(position: Offset, slop: Float): Boolean {
    if (!pending) return false
    val delta = position - checkNotNull(downPosition)
    if (delta.getDistance() <= slop) return false
    if (source == Source.Body && abs(delta.x) <= abs(delta.y)) {
      reset()
      return false
    }
    return true
  }

  fun start(context: EditorGestureContext): Boolean {
    if (
      !pending ||
        !context.isFocused ||
        !context.editing ||
        context.readOnly ||
        !context.mode.canApply(EditorInteractionEvent.CursorDragStart)
    )
      return false
    active = true
    context.reduceMode(EditorInteractionEvent.CursorDragStart)
    context.effects.setScrollGestureLocked(true)
    context.editor.imeNotificationsPaused = true
    context.semantics.contextMenu.hide()
    return true
  }

  fun update(position: Offset, context: EditorGestureContext): Boolean {
    if (!active || context.mode != EditorInteractionMode.CursorDragging) return false
    val cursorPosition = position + grabOffset
    context.semantics.magnifier.show(cursorPosition)
    context.semantics.edgeAutoScroll.trackCursorMove(
      edgePosition = position,
      dispatchPosition = cursorPosition,
      context = context,
      dispatch = { point -> dispatchCursorMove(point, context) },
    )
    val point = context.geometry.resolvePoint(positionInNode = cursorPosition) ?: return true
    if (point.page >= 0) dispatchCursorMove(point, context)
    return true
  }

  private fun dispatchCursorMove(point: PagePoint, context: EditorGestureContext): Boolean {
    val dispatched = context.semantics.pointSelection.enqueueCursorMove(context.editor, point)
    if (dispatched) lastCursorPoint = point
    return dispatched
  }

  fun finish(context: EditorGestureContext, cancelled: Boolean = false): Boolean {
    val consumed = active || pendingHandle
    if (!cancelled && (source == Source.Handle || (source == Source.Body && active))) {
      val point = lastCursorPoint
      val state =
        if (active && point != null) {
          context.semantics.pointSelection.applySelection(
            context.editor,
            SelectionOp.SetAt(point.page, point.x, point.y),
          )
        } else context.editor.publishedState
      if (state != null) context.cursorHandle.show(state, context.effects)
      else context.cursorHandle.hide()
      val showMenu =
        source == Source.Handle &&
          if (active) menuWasVisible else handleTapEnabled && !menuWasVisible
      if (state != null && showMenu) {
        context.semantics.contextMenu.requestShowForAppliedSelection(
          context.editor,
          state,
          allowCollapsed = true,
        )
      }
    }
    if (source == Source.Handle) context.effects.setScrollGestureLocked(false)
    if (active) {
      context.reduceMode(EditorInteractionEvent.CursorDragEnd)
      context.effects.setScrollGestureLocked(false)
      context.editor.imeNotificationsPaused = false
      context.semantics.edgeAutoScroll.stop()
      context.semantics.magnifier.hide()
    }
    reset()
    return consumed
  }

  fun reset() {
    downPosition = null
    grabOffset = Offset.Zero
    lastCursorPoint = null
    source = Source.Caret
    menuWasVisible = false
    handleTapEnabled = false
    active = false
  }

  private enum class Source {
    Caret,
    Body,
    Handle,
  }
}
