package co.typie.editor.interaction.gestures

import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerInputChange
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.SelectionPointUnit
import co.typie.editor.interaction.EditorGestureContext
import co.typie.editor.interaction.EditorInteractionEvent
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.semantics.EditorInteractiveTapResult
import co.typie.platform.Platform
import kotlinx.coroutines.Job
import kotlinx.coroutines.currentCoroutineContext

internal enum class EditorMouseButton {
  Primary,
  Secondary,
  Other,
}

/** Click recognition and selection dragging; touch recognition and presentation stay separate. */
internal class EditorMouseGesture {
  private var drag: Drag? = null
  private var previousClick: Click? = null
  var doubleClickTimeoutMillis = 300L
  var dragSlopPx = 5f

  val hasActivePointer: Boolean
    get() = drag != null

  val dragging: Boolean
    get() = drag?.moved == true

  fun isSecondaryClick(
    button: EditorMouseButton,
    modifiers: InputModifiers,
    platform: Platform,
  ): Boolean =
    button == EditorMouseButton.Secondary ||
      (button == EditorMouseButton.Primary && modifiers.ctrl && platform != Platform.Android)

  fun handlePointerDown(
    change: PointerInputChange,
    position: Offset,
    button: EditorMouseButton,
    modifiers: InputModifiers,
    context: EditorGestureContext,
  ): Boolean {
    val secondary = isSecondaryClick(button, modifiers, context.platform)
    if (button != EditorMouseButton.Primary && !secondary) return false
    val point = context.geometry.resolvePoint(position)?.takeIf { it.page >= 0 } ?: return false
    if (context.mode != EditorInteractionMode.Idle) return false
    val editor = context.editor
    context.semantics.contextMenu.hide()
    context.semantics.magnifier.hide()
    if (secondary) {
      previousClick = null
      val selection = editor.appliedState.selection
      val op =
        if (
          selection != null &&
            selection == editor.publishedState.selection &&
            editor.publishedState.selectionHitRects.any { hit ->
              hit.pageIdx == point.page &&
                point.x >= hit.rect.x &&
                point.x <= hit.rect.x + hit.rect.width &&
                point.y >= hit.rect.y &&
                point.y <= hit.rect.y + hit.rect.height
            }
        ) {
          // Preserve the hit range after any older selection commands already in the queue.
          SelectionOp.Set(selection)
        } else {
          SelectionOp.SetAt(point.page, point.x, point.y)
        }
      val state = context.semantics.pointSelection.applySelection(editor, op) ?: return true
      context.semantics.contextMenu.requestShowForAppliedSelection(
        editor = editor,
        state = state,
        pointerPosition = point,
      )
      return true
    }
    val previous = previousClick
    val count =
      if (
        previous != null &&
          change.uptimeMillis - previous.time in 0..doubleClickTimeoutMillis &&
          (position - previous.position).getDistance() <= dragSlopPx
      )
        previous.count % 3 + 1
      else 1
    previousClick = Click(position, change.uptimeMillis, count)
    if (
      context.semantics.interactiveHit.handleTap(
        editor,
        point,
        editing = context.editing,
        readOnly = context.readOnly,
      ) != EditorInteractiveTapResult.None
    ) {
      previousClick = null
      return true
    }
    if (!context.readOnly && (context.editing || context.effects.requestEditing(editor))) {
      context.effects.requestFocus(editor)
    }
    val original = editor.appliedState.selection
    val op =
      when {
        count > 1 ->
          SelectionOp.SelectUnitAt(
            point.page,
            point.x,
            point.y,
            if (count == 2) SelectionPointUnit.Word else SelectionPointUnit.Paragraph,
          )
        modifiers.shift && original != null ->
          SelectionOp.ExtendTo(
            anchor = original.anchor,
            headPage = point.page,
            headX = point.x,
            headY = point.y,
            baseSelection = null,
            allowCollapse = true,
          )
        else -> SelectionOp.SetAt(point.page, point.x, point.y)
      }
    val applied = context.semantics.pointSelection.applySelection(editor, op) ?: return true
    val selected = applied.selection ?: return true
    drag =
      Drag(
        pointerId = change.id.value,
        down = position,
        anchor =
          if (count == 1 && modifiers.shift && original != null) original.anchor
          else selected.anchor,
        baseSelection = selected.takeIf { count > 1 && !it.isCollapsed() },
      )
    context.reduceMode(EditorInteractionEvent.MouseSelectionStart)
    if (!context.readOnly) context.effects.requestPointerSelectionHead(applied.version)
    return true
  }

  fun handlePointerMove(
    change: PointerInputChange,
    position: Offset?,
    context: EditorGestureContext,
  ): Boolean {
    val current = drag?.takeIf { it.pointerId == change.id.value } ?: return false
    if (position == null) {
      cancel(context)
      return true
    }
    if (!current.moved && (position - current.down).getDistance() < dragSlopPx) return true
    current.moved = true
    previousClick = null
    current.pendingPosition = position
    if (!current.framePending) {
      current.framePending = true
      context.effects.launchInteraction {
        if (drag !== current) return@launchInteraction
        current.frameJob = currentCoroutineContext()[Job]
        withFrameNanos {}
        current.frameJob = null
        current.framePending = false
        val latest = current.pendingPosition
        current.pendingPosition = null
        if (drag === current && latest != null) {
          extend(latest, current, context)
          context.semantics.edgeAutoScroll.trackSelection(
            edgePosition = latest,
            dispatchPosition = latest,
            context = context,
            dispatch = { scrolled -> extend(scrolled.dispatchPosition, current, context) },
          )
        }
      }
    }
    return true
  }

  fun handlePointerUp(
    change: PointerInputChange,
    position: Offset?,
    context: EditorGestureContext,
  ): Boolean {
    val current = drag?.takeIf { it.pointerId == change.id.value } ?: return false
    if (
      position != null && (current.moved || (position - current.down).getDistance() >= dragSlopPx)
    ) {
      previousClick = null
      extend(position, current, context)
    }
    finish(context)
    return true
  }

  fun cancel(context: EditorGestureContext) {
    previousClick = null
    finish(context)
  }

  fun reset() {
    drag?.frameJob?.cancel()
    drag = null
    previousClick = null
  }

  private fun finish(context: EditorGestureContext) {
    val current = drag ?: return
    current.frameJob?.cancel()
    drag = null
    context.semantics.edgeAutoScroll.stop()
    context.reduceMode(EditorInteractionEvent.MouseSelectionEnd)
  }

  private fun extend(position: Offset, current: Drag, context: EditorGestureContext): Boolean {
    if (drag !== current || context.mode != EditorInteractionMode.MouseSelecting) return false
    val point = context.geometry.resolvePoint(position)?.takeIf { it.page >= 0 } ?: return false
    return context.semantics.pointSelection.applySelection(
      context.editor,
      SelectionOp.ExtendTo(
        anchor = current.anchor,
        headPage = point.page,
        headX = point.x,
        headY = point.y,
        baseSelection = current.baseSelection,
        allowCollapse = current.baseSelection == null,
      ),
    ) != null
  }

  private class Drag(
    val pointerId: Long,
    val down: Offset,
    val anchor: Position,
    val baseSelection: Selection?,
    var moved: Boolean = false,
    var pendingPosition: Offset? = null,
    var framePending: Boolean = false,
    var frameJob: Job? = null,
  )

  private data class Click(val position: Offset, val time: Long, val count: Int)
}
