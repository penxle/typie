package co.typie.editor.interaction

internal enum class EditorInteractionMode {
  Idle,
  Panning,
  ViewportZooming,
  AuxiliaryGesture,
  SelectionHandleDragging,
  TableCellHandleDragging,
  LongPressSelecting,
  LongPressWordSelecting,
  DoubleTapSelecting,
  DndLocal,
  DndExternal,
}

internal val EditorInteractionMode.isDndActive: Boolean
  get() = this == EditorInteractionMode.DndLocal || this == EditorInteractionMode.DndExternal

internal val EditorInteractionMode.isViewportZooming: Boolean
  get() = this == EditorInteractionMode.ViewportZooming

internal val EditorInteractionMode.isAuxiliaryGesture: Boolean
  get() = this == EditorInteractionMode.AuxiliaryGesture

internal val EditorInteractionMode.isSelecting: Boolean
  get() =
    this == EditorInteractionMode.SelectionHandleDragging ||
      this == EditorInteractionMode.TableCellHandleDragging ||
      this == EditorInteractionMode.LongPressSelecting ||
      this == EditorInteractionMode.LongPressWordSelecting ||
      this == EditorInteractionMode.DoubleTapSelecting

internal val EditorInteractionMode.isLongPressing: Boolean
  get() =
    this == EditorInteractionMode.LongPressSelecting ||
      this == EditorInteractionMode.LongPressWordSelecting

internal val EditorInteractionMode.allowsViewportScrollReconcile: Boolean
  get() = !isSelecting && !isDndActive && !isAuxiliaryGesture

internal sealed interface EditorInteractionEvent {
  data object PointerCancel : EditorInteractionEvent

  data object PanStart : EditorInteractionEvent

  data object PanEnd : EditorInteractionEvent

  data object PanCancel : EditorInteractionEvent

  data object LongPressStart : EditorInteractionEvent

  data object LongPressEnd : EditorInteractionEvent

  data object LongPressWordStart : EditorInteractionEvent

  data object LongPressWordEnd : EditorInteractionEvent

  data object ViewportZoomStart : EditorInteractionEvent

  data object ViewportZoomEnd : EditorInteractionEvent

  data object SelectionHandleDragStart : EditorInteractionEvent

  data object SelectionHandleDragEnd : EditorInteractionEvent

  data object DoubleTapDragStart : EditorInteractionEvent

  data object DoubleTapDragEnd : EditorInteractionEvent

  data object TableHandleDragStart : EditorInteractionEvent

  data object TableHandleDragEnd : EditorInteractionEvent

  data object DndEnter : EditorInteractionEvent

  data object DndLeave : EditorInteractionEvent

  data object DndDrop : EditorInteractionEvent

  data object DndSessionEnd : EditorInteractionEvent

  data object AuxiliaryGestureEnd : EditorInteractionEvent

  data class DndStart(val local: Boolean) : EditorInteractionEvent

  data object AuxiliaryGestureStart : EditorInteractionEvent

  data object AuxiliaryGestureUpdate : EditorInteractionEvent

  companion object {
    fun dndStart(local: Boolean): EditorInteractionEvent = DndStart(local)
  }
}

internal fun EditorInteractionMode.canApply(event: EditorInteractionEvent): Boolean =
  when (event) {
    EditorInteractionEvent.PointerCancel -> true
    EditorInteractionEvent.PanStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.PanEnd,
    EditorInteractionEvent.PanCancel -> this == EditorInteractionMode.Panning
    EditorInteractionEvent.LongPressStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.LongPressEnd -> this == EditorInteractionMode.LongPressSelecting
    EditorInteractionEvent.LongPressWordStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.LongPressWordEnd -> this == EditorInteractionMode.LongPressWordSelecting
    EditorInteractionEvent.ViewportZoomStart -> !isDndActive && !isViewportZooming
    EditorInteractionEvent.ViewportZoomEnd -> this == EditorInteractionMode.ViewportZooming
    EditorInteractionEvent.SelectionHandleDragStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.SelectionHandleDragEnd ->
      this == EditorInteractionMode.SelectionHandleDragging
    EditorInteractionEvent.DoubleTapDragStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.DoubleTapDragEnd -> this == EditorInteractionMode.DoubleTapSelecting
    EditorInteractionEvent.TableHandleDragStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.TableHandleDragEnd ->
      this == EditorInteractionMode.TableCellHandleDragging
    EditorInteractionEvent.DndEnter -> this != EditorInteractionMode.DndLocal
    EditorInteractionEvent.DndLeave -> this == EditorInteractionMode.DndExternal
    EditorInteractionEvent.DndDrop,
    EditorInteractionEvent.DndSessionEnd -> isDndActive
    is EditorInteractionEvent.DndStart -> !isSelecting
    EditorInteractionEvent.AuxiliaryGestureStart -> this == EditorInteractionMode.Idle
    EditorInteractionEvent.AuxiliaryGestureUpdate -> isAuxiliaryGesture
    EditorInteractionEvent.AuxiliaryGestureEnd -> isAuxiliaryGesture
  }

internal fun EditorInteractionMode.reduce(event: EditorInteractionEvent): EditorInteractionMode {
  if (!canApply(event)) {
    return this
  }

  var mode =
    if (event == EditorInteractionEvent.PointerCancel) {
      EditorInteractionMode.Idle
    } else {
      this
    }

  mode = reduceDnd(mode = mode, event = event)

  if (!mode.isDndActive) {
    mode = reduceViewportZoom(mode = mode, event = event)

    if (!mode.isViewportZooming) {
      mode = reduceAuxiliary(mode = mode, event = event)
      mode = reduceTable(mode = mode, event = event)
      mode = reduceSelection(mode = mode, event = event)
      mode = reducePan(mode = mode, event = event)
    }
  }

  return mode
}

private fun reducePan(
  mode: EditorInteractionMode,
  event: EditorInteractionEvent,
): EditorInteractionMode =
  when {
    event == EditorInteractionEvent.PanStart && mode == EditorInteractionMode.Idle ->
      EditorInteractionMode.Panning
    (event == EditorInteractionEvent.PanEnd || event == EditorInteractionEvent.PanCancel) &&
      mode == EditorInteractionMode.Panning -> EditorInteractionMode.Idle
    else -> mode
  }

private fun reduceViewportZoom(
  mode: EditorInteractionMode,
  event: EditorInteractionEvent,
): EditorInteractionMode =
  when {
    event == EditorInteractionEvent.ViewportZoomStart &&
      mode != EditorInteractionMode.DndLocal &&
      mode != EditorInteractionMode.DndExternal -> EditorInteractionMode.ViewportZooming
    event == EditorInteractionEvent.ViewportZoomEnd &&
      mode == EditorInteractionMode.ViewportZooming -> EditorInteractionMode.Idle
    else -> mode
  }

private fun reduceSelection(
  mode: EditorInteractionMode,
  event: EditorInteractionEvent,
): EditorInteractionMode =
  when {
    event == EditorInteractionEvent.SelectionHandleDragStart ->
      EditorInteractionMode.SelectionHandleDragging
    event == EditorInteractionEvent.SelectionHandleDragEnd &&
      mode == EditorInteractionMode.SelectionHandleDragging -> EditorInteractionMode.Idle
    event == EditorInteractionEvent.LongPressStart -> EditorInteractionMode.LongPressSelecting
    event == EditorInteractionEvent.LongPressEnd &&
      mode == EditorInteractionMode.LongPressSelecting -> EditorInteractionMode.Idle
    event == EditorInteractionEvent.LongPressWordStart ->
      EditorInteractionMode.LongPressWordSelecting
    event == EditorInteractionEvent.LongPressWordEnd &&
      mode == EditorInteractionMode.LongPressWordSelecting -> EditorInteractionMode.Idle
    event == EditorInteractionEvent.DoubleTapDragStart -> EditorInteractionMode.DoubleTapSelecting
    event == EditorInteractionEvent.DoubleTapDragEnd &&
      mode == EditorInteractionMode.DoubleTapSelecting -> EditorInteractionMode.Idle
    else -> mode
  }

private fun reduceTable(
  mode: EditorInteractionMode,
  event: EditorInteractionEvent,
): EditorInteractionMode =
  when {
    event == EditorInteractionEvent.TableHandleDragStart ->
      EditorInteractionMode.TableCellHandleDragging
    event == EditorInteractionEvent.TableHandleDragEnd &&
      mode == EditorInteractionMode.TableCellHandleDragging -> EditorInteractionMode.Idle
    else -> mode
  }

private fun reduceAuxiliary(
  mode: EditorInteractionMode,
  event: EditorInteractionEvent,
): EditorInteractionMode =
  when (event) {
    EditorInteractionEvent.AuxiliaryGestureStart -> EditorInteractionMode.AuxiliaryGesture
    EditorInteractionEvent.AuxiliaryGestureUpdate -> mode
    EditorInteractionEvent.AuxiliaryGestureEnd ->
      if (mode == EditorInteractionMode.AuxiliaryGesture) {
        EditorInteractionMode.Idle
      } else {
        mode
      }
    else -> mode
  }

private fun reduceDnd(
  mode: EditorInteractionMode,
  event: EditorInteractionEvent,
): EditorInteractionMode =
  when (event) {
    is EditorInteractionEvent.DndStart -> {
      if (mode.isSelecting) {
        mode
      } else if (event.local) {
        EditorInteractionMode.DndLocal
      } else {
        EditorInteractionMode.DndExternal
      }
    }
    EditorInteractionEvent.DndEnter ->
      if (mode == EditorInteractionMode.DndLocal) mode else EditorInteractionMode.DndExternal
    EditorInteractionEvent.DndLeave ->
      if (mode == EditorInteractionMode.DndExternal) EditorInteractionMode.Idle else mode
    EditorInteractionEvent.DndDrop,
    EditorInteractionEvent.DndSessionEnd -> EditorInteractionMode.Idle
    else -> mode
  }
