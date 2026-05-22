package co.typie.editor.gesture

internal enum class EditorInteractionMode {
  Idle,
  Panning,
  Pinching,
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

internal val EditorInteractionMode.isPinching: Boolean
  get() = this == EditorInteractionMode.Pinching

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

internal sealed interface EditorInteractionEvent {
  data object PointerCancel : EditorInteractionEvent

  data object PanStart : EditorInteractionEvent

  data object PanEnd : EditorInteractionEvent

  data object PanCancel : EditorInteractionEvent

  data object LongPressStart : EditorInteractionEvent

  data object LongPressEnd : EditorInteractionEvent

  data object LongPressWordStart : EditorInteractionEvent

  data object LongPressWordEnd : EditorInteractionEvent

  data object PinchStart : EditorInteractionEvent

  data object PinchEnd : EditorInteractionEvent

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
