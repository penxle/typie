package co.typie.editor.interaction.gestures

import androidx.compose.ui.geometry.Offset
import co.typie.editor.ext.isCollapsed
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
  var active = false
    private set

  val pending: Boolean
    get() = downPosition != null && !active

  fun prepare(position: Offset, context: EditorGestureContext) {
    if (
      context.platform != Platform.iOS ||
        !context.pointerType.isDirectTouchInteraction() ||
        !context.isFocused ||
        !context.editing ||
        context.readOnly ||
        !context.mode.canApply(EditorInteractionEvent.CursorDragStart)
    )
      return
    val state = context.editor.publishedState
    if (state.selection == null || !state.selection.isCollapsed()) return
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

  fun shouldStart(position: Offset, slop: Float): Boolean =
    pending && (position - checkNotNull(downPosition)).getDistance() > slop

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
    )
    val point = context.geometry.resolvePoint(positionInNode = cursorPosition) ?: return true
    if (point.page >= 0) context.semantics.pointSelection.enqueueCursorMove(context.editor, point)
    return true
  }

  fun finish(context: EditorGestureContext): Boolean {
    val consumed = active
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
    active = false
  }
}
