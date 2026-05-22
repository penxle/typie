package co.typie.editor.interaction

import co.typie.editor.interaction.gestures.EditorDndGesture
import co.typie.editor.interaction.gestures.EditorDoubleTapDragGesture
import co.typie.editor.interaction.gestures.EditorLongPressGesture
import co.typie.editor.interaction.gestures.EditorPanGesture
import co.typie.editor.interaction.gestures.EditorPinchGesture
import co.typie.editor.interaction.gestures.EditorSelectionHandleGesture
import co.typie.editor.interaction.gestures.EditorTableHandleGesture
import co.typie.editor.interaction.gestures.EditorTapGesture

internal class EditorInteractionGestures(
  val tap: EditorTapGesture,
  val doubleTapDrag: EditorDoubleTapDragGesture = EditorDoubleTapDragGesture(),
  val longPress: EditorLongPressGesture = EditorLongPressGesture(),
  val pan: EditorPanGesture = EditorPanGesture(),
  val pinch: EditorPinchGesture = EditorPinchGesture(),
  val selectionHandle: EditorSelectionHandleGesture = EditorSelectionHandleGesture(),
  val tableHandle: EditorTableHandleGesture = EditorTableHandleGesture(),
  val dnd: EditorDndGesture = EditorDndGesture(),
) {
  fun updateTapSlop(tapSlopPx: Float) {
    tap.updateTapSlop(tapSlopPx)
  }

  fun runtimeRead(mode: EditorInteractionMode): EditorInteractionRuntimeRead =
    EditorInteractionRuntimeRead(
      mode = mode,
      pinchIsPinching = pinch.isPinching,
      pinchPointerCount = tap.pressedPointerCount,
      doubleTapActive = doubleTapDrag.active,
      doubleTapDragging = doubleTapDrag.dragging,
      tableCellHandleDragging = tableHandle.dragging,
      hasPendingSelectionHandleDrag = selectionHandle.pendingDrag,
      hasAnyHandleDrag = selectionHandle.activeDrag || tableHandle.dragging,
      panDragActive = pan.dragActive,
    )

  fun reset() {
    tap.reset()
    doubleTapDrag.reset()
    longPress.reset()
    pan.reset()
    pinch.reset()
    selectionHandle.reset()
    tableHandle.reset()
    dnd.reset()
  }
}
