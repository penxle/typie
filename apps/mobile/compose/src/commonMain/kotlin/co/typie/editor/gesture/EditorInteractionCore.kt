package co.typie.editor.gesture

import androidx.compose.ui.geometry.Offset

internal sealed interface EditorInteractionCommand {
  data object TapDown : EditorInteractionCommand

  data object TapUp : EditorInteractionCommand

  data object TapCancel : EditorInteractionCommand

  data object DoubleTapPrepareDrag : EditorInteractionCommand

  data object DoubleTapStartDrag : EditorInteractionCommand

  data object DoubleTapBeginSelecting : EditorInteractionCommand

  data object LongPressBeginSelecting : EditorInteractionCommand

  data object LongPressUpdate : EditorInteractionCommand

  data object LongPressEnd : EditorInteractionCommand

  data object PanStart : EditorInteractionCommand

  data object PanUpdate : EditorInteractionCommand

  data object PanEnd : EditorInteractionCommand

  data object PanCancel : EditorInteractionCommand

  data object PanResume : EditorInteractionCommand

  data object SelectionHandleStart : EditorInteractionCommand

  data object SelectionHandleBeginDragging : EditorInteractionCommand

  data object SelectionHandleUpdate : EditorInteractionCommand

  data object TableCellHandleBeginDown : EditorInteractionCommand

  data object TableCellHandleBeginDragging : EditorInteractionCommand

  data object TableCellHandleUpdate : EditorInteractionCommand

  data object TableCellHandleEnd : EditorInteractionCommand

  data object DndBeginLocal : EditorInteractionCommand

  data object DndBeginExternal : EditorInteractionCommand

  data object DndHandleDropOver : EditorInteractionCommand

  data object DndHandleDropEnter : EditorInteractionCommand

  data object DndShouldEndOnDropEnded : EditorInteractionCommand

  data object DndPerformDrop : EditorInteractionCommand

  data object AuxiliaryBegin : EditorInteractionCommand

  data object AuxiliaryUpdate : EditorInteractionCommand

  data object AuxiliaryEnd : EditorInteractionCommand

  data class TapDispatch(val page: Int) : EditorInteractionCommand

  data class DoubleTapDispatchSelection(val page: Int) : EditorInteractionCommand

  data class DoubleTapUpdateSelection(val localPosition: Offset, val dragStartPosition: Offset?) :
    EditorInteractionCommand

  data class DoubleTapExtendSelection(val page: Int, val hasSelectionContext: Boolean) :
    EditorInteractionCommand

  data class LongPressStart(val viewportPosition: Offset?) : EditorInteractionCommand

  data class PanApplyRaw(
    val hasPreviousPointerPosition: Boolean,
    val fromPointerSignal: Boolean = false,
  ) : EditorInteractionCommand

  data class SelectionHandleEnd(val hasActiveDrag: Boolean) : EditorInteractionCommand

  data class DndHandleDropOverItem(val hasItem: Boolean) : EditorInteractionCommand

  data class DndPerformDropOnPage(val page: Int) : EditorInteractionCommand
}

internal enum class EditorInteractionBlockReason {
  Pinching,
  DndLocked,
  DoubleTapSelecting,
  SelectionHandlePending,
  Selecting,
  SelectionHandleDragging,
  AuxiliaryGesture,
  PanDragActive,
  PointerTrackMissing,
  NonSinglePointer,
  TableCellHandleDragging,
  ViewportUnavailable,
  SessionAlreadyActive,
  NotActive,
  NoActiveDrag,
  PageOutOfRange,
  DoubleTapDragging,
  DoubleTapActive,
  NotDragging,
  BelowStartThreshold,
  SelectionContextUnavailable,
  MissingDropItem,
  DndInactive,
}

internal data class EditorInteractionRuntimeRead(
  val mode: EditorInteractionMode = EditorInteractionMode.Idle,
  val pinchIsPinching: Boolean = false,
  val pinchPointerCount: Int = 0,
  val doubleTapActive: Boolean = false,
  val doubleTapDragging: Boolean = false,
  val tableCellHandleDragging: Boolean = false,
  val hasPendingSelectionHandleDrag: Boolean = false,
  val hasAnyHandleDrag: Boolean = false,
  val panDragActive: Boolean = false,
) {
  val isPinching: Boolean
    get() = pinchIsPinching || mode.isPinching
}

internal class EditorInteractionCore {
  fun decide(command: EditorInteractionCommand, runtime: EditorInteractionRuntimeRead): Boolean =
    blockReason(command = command, runtime = runtime) == null

  fun blockReason(
    command: EditorInteractionCommand,
    runtime: EditorInteractionRuntimeRead,
  ): EditorInteractionBlockReason? =
    when (command) {
      EditorInteractionCommand.TapDown -> {
        if (runtime.isPinching) EditorInteractionBlockReason.Pinching else null
      }
      EditorInteractionCommand.TapUp -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.doubleTapDragging -> EditorInteractionBlockReason.DoubleTapDragging
          else -> null
        }
      }
      EditorInteractionCommand.TapCancel -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.doubleTapActive -> EditorInteractionBlockReason.DoubleTapActive
          else -> null
        }
      }
      is EditorInteractionCommand.TapDispatch,
      is EditorInteractionCommand.DoubleTapDispatchSelection -> {
        val page =
          when (command) {
            is EditorInteractionCommand.TapDispatch -> command.page
            is EditorInteractionCommand.DoubleTapDispatchSelection -> command.page
          }
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          page < 0 -> EditorInteractionBlockReason.PageOutOfRange
          else -> null
        }
      }
      EditorInteractionCommand.DoubleTapPrepareDrag,
      EditorInteractionCommand.DoubleTapStartDrag,
      EditorInteractionCommand.DoubleTapBeginSelecting -> {
        if (runtime.isPinching) EditorInteractionBlockReason.Pinching else null
      }
      is EditorInteractionCommand.DoubleTapUpdateSelection -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          !runtime.doubleTapDragging -> EditorInteractionBlockReason.NotDragging
          command.dragStartPosition != null &&
            (command.localPosition - command.dragStartPosition).getDistance() < 4f ->
            EditorInteractionBlockReason.BelowStartThreshold
          else -> null
        }
      }
      is EditorInteractionCommand.DoubleTapExtendSelection -> {
        when {
          !command.hasSelectionContext -> EditorInteractionBlockReason.SelectionContextUnavailable
          command.page < 0 -> EditorInteractionBlockReason.PageOutOfRange
          else -> null
        }
      }
      is EditorInteractionCommand.LongPressStart -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.tableCellHandleDragging -> EditorInteractionBlockReason.TableCellHandleDragging
          runtime.doubleTapActive -> EditorInteractionBlockReason.DoubleTapSelecting
          command.viewportPosition == null -> EditorInteractionBlockReason.ViewportUnavailable
          else -> null
        }
      }
      EditorInteractionCommand.LongPressBeginSelecting -> {
        if (runtime.mode.isLongPressing) EditorInteractionBlockReason.SessionAlreadyActive else null
      }
      EditorInteractionCommand.LongPressUpdate,
      EditorInteractionCommand.LongPressEnd -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          !runtime.mode.isLongPressing -> EditorInteractionBlockReason.NotActive
          runtime.doubleTapActive -> EditorInteractionBlockReason.DoubleTapSelecting
          else -> null
        }
      }
      EditorInteractionCommand.PanStart -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.mode.isDndActive -> EditorInteractionBlockReason.DndLocked
          runtime.doubleTapActive -> EditorInteractionBlockReason.DoubleTapSelecting
          runtime.hasPendingSelectionHandleDrag ->
            EditorInteractionBlockReason.SelectionHandlePending
          else -> null
        }
      }
      EditorInteractionCommand.PanUpdate -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.mode.isDndActive -> EditorInteractionBlockReason.DndLocked
          else -> null
        }
      }
      EditorInteractionCommand.PanEnd,
      EditorInteractionCommand.PanCancel -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.mode.isDndActive -> EditorInteractionBlockReason.DndLocked
          runtime.doubleTapActive -> EditorInteractionBlockReason.DoubleTapSelecting
          else -> null
        }
      }
      EditorInteractionCommand.PanResume -> {
        when {
          runtime.mode.isSelecting -> EditorInteractionBlockReason.Selecting
          runtime.doubleTapActive -> EditorInteractionBlockReason.DoubleTapSelecting
          runtime.hasAnyHandleDrag -> EditorInteractionBlockReason.SelectionHandleDragging
          runtime.hasPendingSelectionHandleDrag ->
            EditorInteractionBlockReason.SelectionHandlePending
          else -> null
        }
      }
      is EditorInteractionCommand.PanApplyRaw -> {
        when {
          !command.fromPointerSignal && runtime.pinchPointerCount != 1 ->
            EditorInteractionBlockReason.NonSinglePointer
          !command.fromPointerSignal && !command.hasPreviousPointerPosition ->
            EditorInteractionBlockReason.PointerTrackMissing
          runtime.mode.isSelecting -> EditorInteractionBlockReason.Selecting
          runtime.mode.isAuxiliaryGesture -> EditorInteractionBlockReason.AuxiliaryGesture
          runtime.hasAnyHandleDrag -> EditorInteractionBlockReason.SelectionHandleDragging
          runtime.hasPendingSelectionHandleDrag ->
            EditorInteractionBlockReason.SelectionHandlePending
          !command.fromPointerSignal && runtime.panDragActive ->
            EditorInteractionBlockReason.PanDragActive
          else -> null
        }
      }
      EditorInteractionCommand.SelectionHandleStart -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.mode.isDndActive -> EditorInteractionBlockReason.DndLocked
          runtime.tableCellHandleDragging -> EditorInteractionBlockReason.TableCellHandleDragging
          else -> null
        }
      }
      EditorInteractionCommand.SelectionHandleBeginDragging -> {
        if (runtime.isPinching) EditorInteractionBlockReason.Pinching else null
      }
      EditorInteractionCommand.SelectionHandleUpdate -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.mode != EditorInteractionMode.SelectionHandleDragging ->
            EditorInteractionBlockReason.NotActive
          else -> null
        }
      }
      is EditorInteractionCommand.SelectionHandleEnd -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.tableCellHandleDragging -> EditorInteractionBlockReason.TableCellHandleDragging
          !command.hasActiveDrag -> EditorInteractionBlockReason.NoActiveDrag
          else -> null
        }
      }
      EditorInteractionCommand.TableCellHandleBeginDown -> {
        when {
          runtime.isPinching -> EditorInteractionBlockReason.Pinching
          runtime.mode.isDndActive -> EditorInteractionBlockReason.DndLocked
          else -> null
        }
      }
      EditorInteractionCommand.TableCellHandleBeginDragging -> null
      EditorInteractionCommand.TableCellHandleUpdate,
      EditorInteractionCommand.TableCellHandleEnd -> {
        if (runtime.mode != EditorInteractionMode.TableCellHandleDragging) {
          EditorInteractionBlockReason.NotActive
        } else {
          null
        }
      }
      EditorInteractionCommand.DndBeginLocal,
      EditorInteractionCommand.DndBeginExternal -> null
      EditorInteractionCommand.DndHandleDropOver,
      EditorInteractionCommand.DndHandleDropEnter,
      EditorInteractionCommand.DndPerformDrop -> {
        if (runtime.isPinching) EditorInteractionBlockReason.Pinching else null
      }
      is EditorInteractionCommand.DndHandleDropOverItem -> {
        if (!command.hasItem) EditorInteractionBlockReason.MissingDropItem else null
      }
      EditorInteractionCommand.DndShouldEndOnDropEnded -> {
        if (!runtime.mode.isDndActive) EditorInteractionBlockReason.DndInactive else null
      }
      is EditorInteractionCommand.DndPerformDropOnPage -> {
        if (command.page < 0) EditorInteractionBlockReason.PageOutOfRange else null
      }
      EditorInteractionCommand.AuxiliaryBegin -> null
      EditorInteractionCommand.AuxiliaryUpdate,
      EditorInteractionCommand.AuxiliaryEnd -> {
        if (!runtime.mode.isAuxiliaryGesture) EditorInteractionBlockReason.NotActive else null
      }
    }

  fun reduce(
    previous: EditorInteractionMode,
    event: EditorInteractionEvent,
  ): EditorInteractionMode {
    var mode =
      if (event == EditorInteractionEvent.PointerCancel) {
        EditorInteractionMode.Idle
      } else {
        previous
      }

    mode = reduceDnd(mode = mode, event = event)

    if (!mode.isDndActive) {
      mode = reducePinch(mode = mode, event = event)

      if (!mode.isPinching) {
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

  private fun reducePinch(
    mode: EditorInteractionMode,
    event: EditorInteractionEvent,
  ): EditorInteractionMode =
    when {
      event == EditorInteractionEvent.PinchStart &&
        mode != EditorInteractionMode.DndLocal &&
        mode != EditorInteractionMode.DndExternal -> EditorInteractionMode.Pinching
      event == EditorInteractionEvent.PinchEnd && mode == EditorInteractionMode.Pinching ->
        EditorInteractionMode.Idle
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
}
