package co.typie.editor.gesture

internal class EditorInteractionGestureSet(
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

internal class EditorDoubleTapDragGesture {
  val active: Boolean = false
  val dragging: Boolean = false

  fun reset() {
    // TODO(editor-parity): port double-tap word-selection drag state.
  }
}

internal class EditorLongPressGesture {
  fun reset() {
    // TODO(editor-parity): port long-press selection and magnifier state.
  }
}

internal class EditorPanGesture {
  val dragActive: Boolean = false

  fun reset() {
    // TODO(editor-parity): port raw viewport pan admission and resume state.
  }
}

internal class EditorPinchGesture {
  val isPinching: Boolean = false

  fun reset() {
    // TODO(editor-parity): port pinch viewport state once zoom gestures move here.
  }
}

internal class EditorSelectionHandleGesture {
  val pendingDrag: Boolean = false
  val activeDrag: Boolean = false

  fun reset() {
    // TODO(editor-parity): port selection-handle drag state after handle geometry exists.
  }
}

internal class EditorTableHandleGesture {
  val dragging: Boolean = false

  fun reset() {
    // TODO(editor-parity): port table-cell and column handle drag state.
  }
}

internal class EditorDndGesture {
  fun reset() {
    // TODO(editor-parity): port local/external DND drop-state dispatch.
  }
}
