import 'package:flutter/material.dart';

import 'state.dart';

enum InteractionEventType {
  pointerDown,
  pointerMove,
  pointerUp,
  pointerCancel,
  panStart,
  panEnd,
  panCancel,
  longPressStart,
  longPressEnd,
  pinchStart,
  pinchEnd,
  selectionHandleDragStart,
  selectionHandleDragEnd,
  doubleTapDragStart,
  doubleTapDragEnd,
  tableHandleDragStart,
  tableHandleDragEnd,
  dndStart,
  dndEnter,
  dndOver,
  dndLeave,
  dndDrop,
  dndSessionEnd,
  auxiliaryGestureStart,
  auxiliaryGestureUpdate,
  auxiliaryGestureEnd,
}

class InteractionEvent {
  const InteractionEvent(this.type, {this.local, this.kind});
  const InteractionEvent.dndStart({required bool local}) : this(InteractionEventType.dndStart, local: local);
  const InteractionEvent.auxiliaryGestureStart({required AuxiliaryGestureKind kind})
    : this(InteractionEventType.auxiliaryGestureStart, kind: kind);
  const InteractionEvent.auxiliaryGestureUpdate({required AuxiliaryGestureKind kind})
    : this(InteractionEventType.auxiliaryGestureUpdate, kind: kind);

  final InteractionEventType type;
  final bool? local;
  final AuxiliaryGestureKind? kind;

  static const pointerDown = InteractionEvent(InteractionEventType.pointerDown);
  static const pointerMove = InteractionEvent(InteractionEventType.pointerMove);
  static const pointerUp = InteractionEvent(InteractionEventType.pointerUp);
  static const pointerCancel = InteractionEvent(InteractionEventType.pointerCancel);

  static const panStart = InteractionEvent(InteractionEventType.panStart);
  static const panEnd = InteractionEvent(InteractionEventType.panEnd);
  static const panCancel = InteractionEvent(InteractionEventType.panCancel);

  static const longPressStart = InteractionEvent(InteractionEventType.longPressStart);
  static const longPressEnd = InteractionEvent(InteractionEventType.longPressEnd);

  static const pinchStart = InteractionEvent(InteractionEventType.pinchStart);
  static const pinchEnd = InteractionEvent(InteractionEventType.pinchEnd);

  static const selectionHandleDragStart = InteractionEvent(InteractionEventType.selectionHandleDragStart);
  static const selectionHandleDragEnd = InteractionEvent(InteractionEventType.selectionHandleDragEnd);

  static const doubleTapDragStart = InteractionEvent(InteractionEventType.doubleTapDragStart);
  static const doubleTapDragEnd = InteractionEvent(InteractionEventType.doubleTapDragEnd);

  static const tableHandleDragStart = InteractionEvent(InteractionEventType.tableHandleDragStart);
  static const tableHandleDragEnd = InteractionEvent(InteractionEventType.tableHandleDragEnd);

  static const dndEnter = InteractionEvent(InteractionEventType.dndEnter);
  static const dndOver = InteractionEvent(InteractionEventType.dndOver);
  static const dndLeave = InteractionEvent(InteractionEventType.dndLeave);
  static const dndDrop = InteractionEvent(InteractionEventType.dndDrop);
  static const dndSessionEnd = InteractionEvent(InteractionEventType.dndSessionEnd);

  static const auxiliaryGestureEnd = InteractionEvent(InteractionEventType.auxiliaryGestureEnd);
}

enum InteractionCommandCategory {
  tap,
  doubleTapDrag,
  longPress,
  pan,
  selectionHandle,
  tableCellHandle,
  dnd,
  auxiliaryGesture,
}

enum InteractionCommandType {
  tapDown,
  tapUp,
  tapCancel,
  tapDispatch,
  doubleTapDispatchSelection,
  doubleTapPrepareDrag,
  doubleTapStartDrag,
  doubleTapBeginSelecting,
  doubleTapUpdateSelection,
  doubleTapExtendSelection,
  longPressStart,
  longPressBeginSelecting,
  longPressUpdate,
  longPressEnd,
  panStart,
  panUpdate,
  panEnd,
  panCancel,
  panResume,
  panApplyRaw,
  selectionHandleStart,
  selectionHandleBeginDragging,
  selectionHandleUpdate,
  selectionHandleEnd,
  tableCellHandleBeginDown,
  tableCellHandleBeginDragging,
  tableCellHandleUpdate,
  tableCellHandleEnd,
  dndBeginLocal,
  dndHandleDropOver,
  dndHandleDropOverItem,
  dndHandleDropEnter,
  dndBeginExternal,
  dndShouldEndOnDropEnded,
  dndPerformDrop,
  dndPerformDropOnPage,
  auxiliaryBegin,
  auxiliaryUpdate,
  auxiliaryEnd,
}

enum InteractionBlockReason {
  pinching,
  dndLocked,
  doubleTapSelecting,
  selectionHandlePending,
  selecting,
  selectionHandleDragging,
  auxiliaryGesture,
  panDragActive,
  pointerTrackMissing,
  nonSinglePointer,
  tableCellHandleDragging,
  viewportUnavailable,
  sessionAlreadyActive,
  notActive,
  modeRejected,
  noActiveDrag,
  pageOutOfRange,
  doubleTapDragging,
  doubleTapActive,
  notDragging,
  belowStartThreshold,
  selectionContextUnavailable,
  missingDropItem,
  dndInactive,
}

class InteractionCommand {
  const InteractionCommand.tapDispatch({required int pageIdx})
    : this._(InteractionCommandType.tapDispatch, pageIdx: pageIdx);

  const InteractionCommand.doubleTapDispatchSelection({required int pageIdx})
    : this._(InteractionCommandType.doubleTapDispatchSelection, pageIdx: pageIdx);

  const InteractionCommand.doubleTapUpdateSelection({required Offset localPosition, required Offset? dragStartPosition})
    : this._(
        InteractionCommandType.doubleTapUpdateSelection,
        localPosition: localPosition,
        dragStartPosition: dragStartPosition,
      );

  const InteractionCommand.doubleTapExtendSelection({required int pageIdx, required bool hasSelectionContext})
    : this._(
        InteractionCommandType.doubleTapExtendSelection,
        pageIdx: pageIdx,
        hasSelectionContext: hasSelectionContext,
      );

  const InteractionCommand.longPressStart({required Offset? viewportPosition})
    : this._(InteractionCommandType.longPressStart, viewportPosition: viewportPosition);

  const InteractionCommand.panApplyRaw({required bool hasPreviousPointerPosition})
    : this._(InteractionCommandType.panApplyRaw, hasPreviousPointerPosition: hasPreviousPointerPosition);
  const InteractionCommand.selectionHandleEnd({required bool hasActiveDrag})
    : this._(InteractionCommandType.selectionHandleEnd, hasActiveDrag: hasActiveDrag);
  const InteractionCommand.dndHandleDropOverItem({required bool hasItem})
    : this._(InteractionCommandType.dndHandleDropOverItem, hasItem: hasItem);
  const InteractionCommand.dndPerformDropOnPage({required int pageIdx})
    : this._(InteractionCommandType.dndPerformDropOnPage, pageIdx: pageIdx);
  const InteractionCommand._(
    this.type, {
    this.pageIdx,
    this.hasItem,
    this.hasSelectionContext,
    this.viewportPosition,
    this.hasActiveDrag,
    this.hasPreviousPointerPosition,
    this.localPosition,
    this.dragStartPosition,
  });

  final InteractionCommandType type;
  final int? pageIdx;
  final bool? hasItem;
  final bool? hasSelectionContext;
  final Offset? viewportPosition;
  final bool? hasActiveDrag;
  final bool? hasPreviousPointerPosition;
  final Offset? localPosition;
  final Offset? dragStartPosition;

  InteractionCommandCategory get category {
    switch (type) {
      case InteractionCommandType.tapDown:
      case InteractionCommandType.tapUp:
      case InteractionCommandType.tapCancel:
      case InteractionCommandType.tapDispatch:
        return InteractionCommandCategory.tap;
      case InteractionCommandType.doubleTapDispatchSelection:
      case InteractionCommandType.doubleTapPrepareDrag:
      case InteractionCommandType.doubleTapStartDrag:
      case InteractionCommandType.doubleTapBeginSelecting:
      case InteractionCommandType.doubleTapUpdateSelection:
      case InteractionCommandType.doubleTapExtendSelection:
        return InteractionCommandCategory.doubleTapDrag;
      case InteractionCommandType.longPressStart:
      case InteractionCommandType.longPressBeginSelecting:
      case InteractionCommandType.longPressUpdate:
      case InteractionCommandType.longPressEnd:
        return InteractionCommandCategory.longPress;
      case InteractionCommandType.panStart:
      case InteractionCommandType.panUpdate:
      case InteractionCommandType.panEnd:
      case InteractionCommandType.panCancel:
      case InteractionCommandType.panResume:
      case InteractionCommandType.panApplyRaw:
        return InteractionCommandCategory.pan;
      case InteractionCommandType.selectionHandleStart:
      case InteractionCommandType.selectionHandleBeginDragging:
      case InteractionCommandType.selectionHandleUpdate:
      case InteractionCommandType.selectionHandleEnd:
        return InteractionCommandCategory.selectionHandle;
      case InteractionCommandType.tableCellHandleBeginDown:
      case InteractionCommandType.tableCellHandleBeginDragging:
      case InteractionCommandType.tableCellHandleUpdate:
      case InteractionCommandType.tableCellHandleEnd:
        return InteractionCommandCategory.tableCellHandle;
      case InteractionCommandType.dndBeginLocal:
      case InteractionCommandType.dndHandleDropOver:
      case InteractionCommandType.dndHandleDropOverItem:
      case InteractionCommandType.dndHandleDropEnter:
      case InteractionCommandType.dndBeginExternal:
      case InteractionCommandType.dndShouldEndOnDropEnded:
      case InteractionCommandType.dndPerformDrop:
      case InteractionCommandType.dndPerformDropOnPage:
        return InteractionCommandCategory.dnd;
      case InteractionCommandType.auxiliaryBegin:
      case InteractionCommandType.auxiliaryUpdate:
      case InteractionCommandType.auxiliaryEnd:
        return InteractionCommandCategory.auxiliaryGesture;
    }
  }

  static const tapDown = InteractionCommand._(InteractionCommandType.tapDown);
  static const tapUp = InteractionCommand._(InteractionCommandType.tapUp);
  static const tapCancel = InteractionCommand._(InteractionCommandType.tapCancel);

  static const doubleTapPrepareDrag = InteractionCommand._(InteractionCommandType.doubleTapPrepareDrag);
  static const doubleTapStartDrag = InteractionCommand._(InteractionCommandType.doubleTapStartDrag);
  static const doubleTapBeginSelecting = InteractionCommand._(InteractionCommandType.doubleTapBeginSelecting);

  static const longPressBeginSelecting = InteractionCommand._(InteractionCommandType.longPressBeginSelecting);
  static const longPressUpdate = InteractionCommand._(InteractionCommandType.longPressUpdate);
  static const longPressEnd = InteractionCommand._(InteractionCommandType.longPressEnd);

  static const panStart = InteractionCommand._(InteractionCommandType.panStart);
  static const panUpdate = InteractionCommand._(InteractionCommandType.panUpdate);
  static const panEnd = InteractionCommand._(InteractionCommandType.panEnd);
  static const panCancel = InteractionCommand._(InteractionCommandType.panCancel);
  static const panResume = InteractionCommand._(InteractionCommandType.panResume);

  static const selectionHandleStart = InteractionCommand._(InteractionCommandType.selectionHandleStart);
  static const selectionHandleBeginDragging = InteractionCommand._(InteractionCommandType.selectionHandleBeginDragging);
  static const selectionHandleUpdate = InteractionCommand._(InteractionCommandType.selectionHandleUpdate);

  static const tableCellHandleBeginDown = InteractionCommand._(InteractionCommandType.tableCellHandleBeginDown);
  static const tableCellHandleBeginDragging = InteractionCommand._(InteractionCommandType.tableCellHandleBeginDragging);
  static const tableCellHandleUpdate = InteractionCommand._(InteractionCommandType.tableCellHandleUpdate);
  static const tableCellHandleEnd = InteractionCommand._(InteractionCommandType.tableCellHandleEnd);

  static const dndBeginLocal = InteractionCommand._(InteractionCommandType.dndBeginLocal);
  static const dndHandleDropOver = InteractionCommand._(InteractionCommandType.dndHandleDropOver);

  static const dndHandleDropEnter = InteractionCommand._(InteractionCommandType.dndHandleDropEnter);
  static const dndBeginExternal = InteractionCommand._(InteractionCommandType.dndBeginExternal);
  static const dndShouldEndOnDropEnded = InteractionCommand._(InteractionCommandType.dndShouldEndOnDropEnded);
  static const dndPerformDrop = InteractionCommand._(InteractionCommandType.dndPerformDrop);

  static const auxiliaryBegin = InteractionCommand._(InteractionCommandType.auxiliaryBegin);
  static const auxiliaryUpdate = InteractionCommand._(InteractionCommandType.auxiliaryUpdate);
  static const auxiliaryEnd = InteractionCommand._(InteractionCommandType.auxiliaryEnd);
}

class InteractionRuntimeRead {
  const InteractionRuntimeRead({
    required this.snapshot,
    required this.pinchIsPinching,
    required this.pinchPointerCount,
    required this.doubleTapActive,
    required this.doubleTapDragging,
    required this.tableCellHandleDragging,
    required this.longPressActive,
    required this.hasPendingSelectionHandleDrag,
    required this.hasAnyHandleDrag,
    required this.panDragActive,
    required this.dndLocked,
  });

  final InteractionSnapshot snapshot;
  final bool pinchIsPinching;
  final int pinchPointerCount;
  final bool doubleTapActive;
  final bool doubleTapDragging;
  final bool tableCellHandleDragging;
  final bool longPressActive;
  final bool hasPendingSelectionHandleDrag;
  final bool hasAnyHandleDrag;
  final bool panDragActive;
  final bool dndLocked;
}

class InteractionDecision {
  const InteractionDecision({required this.allowed, this.reason, this.nextSnapshot});

  final bool allowed;
  final InteractionBlockReason? reason;
  final InteractionSnapshot? nextSnapshot;
}

class InteractionCore {
  const InteractionCore();

  InteractionDecision decide({required InteractionCommand command, required InteractionRuntimeRead runtime}) {
    final reason = _blockReason(command: command, runtime: runtime);
    return InteractionDecision(allowed: reason == null, reason: reason);
  }

  InteractionSnapshot reduce({required InteractionSnapshot previous, required InteractionEvent event}) {
    var mode = previous.mode;
    var auxiliaryKind = previous.auxiliaryGestureKind;

    if (event.type == InteractionEventType.pointerCancel) {
      mode = InteractionMode.idle;
      auxiliaryKind = null;
    }

    final modeBeforeDnd = mode;
    mode = _reduceDnd(mode: mode, event: event);
    if (mode != modeBeforeDnd && mode != InteractionMode.auxiliaryGesture) {
      auxiliaryKind = null;
    }

    if (mode != InteractionMode.dndLocal && mode != InteractionMode.dndExternal) {
      mode = _reducePinch(mode: mode, event: event);
      if (mode == InteractionMode.pinching) {
        auxiliaryKind = null;
      }

      if (mode != InteractionMode.pinching) {
        final auxiliary = _reduceAuxiliary(mode: mode, kind: auxiliaryKind, event: event);
        mode = auxiliary.mode;
        auxiliaryKind = auxiliary.kind;

        mode = _reduceTable(mode: mode, event: event);
        mode = _reduceSelection(mode: mode, event: event);
        mode = _reducePan(mode: mode, event: event);
      }
    }

    return previous.copyWith(
      mode: mode,
      auxiliaryGestureKind: auxiliaryKind,
      clearAuxiliaryGestureKind: mode != InteractionMode.auxiliaryGesture,
    );
  }

  InteractionBlockReason? _blockReason({required InteractionCommand command, required InteractionRuntimeRead runtime}) {
    switch (command.type) {
      case InteractionCommandType.tapDown:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        return null;
      case InteractionCommandType.tapUp:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.doubleTapDragging) {
          return InteractionBlockReason.doubleTapDragging;
        }
        return null;
      case InteractionCommandType.tapCancel:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.doubleTapActive) {
          return InteractionBlockReason.doubleTapActive;
        }
        return null;
      case InteractionCommandType.tapDispatch:
      case InteractionCommandType.doubleTapDispatchSelection:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if ((command.pageIdx ?? -1) < 0) {
          return InteractionBlockReason.pageOutOfRange;
        }
        return null;
      case InteractionCommandType.doubleTapPrepareDrag:
      case InteractionCommandType.doubleTapStartDrag:
      case InteractionCommandType.doubleTapBeginSelecting:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        return null;
      case InteractionCommandType.doubleTapUpdateSelection:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (!runtime.doubleTapDragging) {
          return InteractionBlockReason.notDragging;
        }
        final localPosition = command.localPosition;
        final dragStartPosition = command.dragStartPosition;
        if (localPosition != null && dragStartPosition != null && (localPosition - dragStartPosition).distance < 4) {
          return InteractionBlockReason.belowStartThreshold;
        }
        return null;
      case InteractionCommandType.doubleTapExtendSelection:
        if (!(command.hasSelectionContext ?? false)) {
          return InteractionBlockReason.selectionContextUnavailable;
        }
        if ((command.pageIdx ?? -1) < 0) {
          return InteractionBlockReason.pageOutOfRange;
        }
        return null;
      case InteractionCommandType.longPressStart:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.tableCellHandleDragging) {
          return InteractionBlockReason.tableCellHandleDragging;
        }
        if (runtime.doubleTapActive) {
          return InteractionBlockReason.doubleTapSelecting;
        }
        if (command.viewportPosition == null) {
          return InteractionBlockReason.viewportUnavailable;
        }
        return null;
      case InteractionCommandType.longPressBeginSelecting:
        if (runtime.longPressActive) {
          return InteractionBlockReason.sessionAlreadyActive;
        }
        return null;
      case InteractionCommandType.longPressUpdate:
      case InteractionCommandType.longPressEnd:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (!runtime.longPressActive) {
          return InteractionBlockReason.notActive;
        }
        if (runtime.doubleTapActive) {
          return InteractionBlockReason.doubleTapSelecting;
        }
        return null;
      case InteractionCommandType.panStart:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.dndLocked) {
          return InteractionBlockReason.dndLocked;
        }
        if (runtime.doubleTapActive) {
          return InteractionBlockReason.doubleTapSelecting;
        }
        if (runtime.hasPendingSelectionHandleDrag) {
          return InteractionBlockReason.selectionHandlePending;
        }
        return null;
      case InteractionCommandType.panUpdate:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.dndLocked) {
          return InteractionBlockReason.dndLocked;
        }
        return null;
      case InteractionCommandType.panEnd:
      case InteractionCommandType.panCancel:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.dndLocked) {
          return InteractionBlockReason.dndLocked;
        }
        if (runtime.doubleTapActive) {
          return InteractionBlockReason.doubleTapSelecting;
        }
        return null;
      case InteractionCommandType.panResume:
        if (runtime.snapshot.isSelecting) {
          return InteractionBlockReason.selecting;
        }
        if (runtime.doubleTapActive) {
          return InteractionBlockReason.doubleTapSelecting;
        }
        if (runtime.hasAnyHandleDrag) {
          return InteractionBlockReason.selectionHandleDragging;
        }
        if (runtime.hasPendingSelectionHandleDrag) {
          return InteractionBlockReason.selectionHandlePending;
        }
        return null;
      case InteractionCommandType.panApplyRaw:
        if (runtime.pinchPointerCount != 1) {
          return InteractionBlockReason.nonSinglePointer;
        }
        if (!(command.hasPreviousPointerPosition ?? false)) {
          return InteractionBlockReason.pointerTrackMissing;
        }
        if (runtime.snapshot.isSelecting) {
          return InteractionBlockReason.selecting;
        }
        if (runtime.snapshot.isAuxiliaryGesture) {
          return InteractionBlockReason.auxiliaryGesture;
        }
        if (runtime.hasAnyHandleDrag) {
          return InteractionBlockReason.selectionHandleDragging;
        }
        if (runtime.hasPendingSelectionHandleDrag) {
          return InteractionBlockReason.selectionHandlePending;
        }
        if (runtime.panDragActive) {
          return InteractionBlockReason.panDragActive;
        }
        return null;
      case InteractionCommandType.selectionHandleStart:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.dndLocked) {
          return InteractionBlockReason.dndLocked;
        }
        if (runtime.tableCellHandleDragging) {
          return InteractionBlockReason.tableCellHandleDragging;
        }
        return null;
      case InteractionCommandType.selectionHandleBeginDragging:
      case InteractionCommandType.selectionHandleUpdate:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        return null;
      case InteractionCommandType.selectionHandleEnd:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.tableCellHandleDragging) {
          return InteractionBlockReason.tableCellHandleDragging;
        }
        if (!(command.hasActiveDrag ?? false)) {
          return InteractionBlockReason.noActiveDrag;
        }
        return null;
      case InteractionCommandType.tableCellHandleBeginDown:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        if (runtime.dndLocked) {
          return InteractionBlockReason.dndLocked;
        }
        return null;
      case InteractionCommandType.tableCellHandleBeginDragging:
        return null;
      case InteractionCommandType.tableCellHandleUpdate:
      case InteractionCommandType.tableCellHandleEnd:
        if (runtime.snapshot.mode != InteractionMode.tableCellHandleDragging) {
          return InteractionBlockReason.notActive;
        }
        return null;
      case InteractionCommandType.dndBeginLocal:
      case InteractionCommandType.dndBeginExternal:
        return null;
      case InteractionCommandType.dndHandleDropOver:
      case InteractionCommandType.dndHandleDropEnter:
      case InteractionCommandType.dndPerformDrop:
        if (runtime.pinchIsPinching) {
          return InteractionBlockReason.pinching;
        }
        return null;
      case InteractionCommandType.dndHandleDropOverItem:
        if (!(command.hasItem ?? false)) {
          return InteractionBlockReason.missingDropItem;
        }
        return null;
      case InteractionCommandType.dndShouldEndOnDropEnded:
        if (!runtime.snapshot.isDndActive) {
          return InteractionBlockReason.dndInactive;
        }
        return null;
      case InteractionCommandType.dndPerformDropOnPage:
        if ((command.pageIdx ?? -1) < 0) {
          return InteractionBlockReason.pageOutOfRange;
        }
        return null;
      case InteractionCommandType.auxiliaryBegin:
        return null;
      case InteractionCommandType.auxiliaryUpdate:
      case InteractionCommandType.auxiliaryEnd:
        if (!runtime.snapshot.isAuxiliaryGesture) {
          return InteractionBlockReason.notActive;
        }
        return null;
    }
  }

  InteractionMode _reducePan({required InteractionMode mode, required InteractionEvent event}) {
    if (event.type == InteractionEventType.panStart) {
      if (mode == InteractionMode.idle) {
        return InteractionMode.panning;
      }
      return mode;
    }

    if (event.type == InteractionEventType.panEnd || event.type == InteractionEventType.panCancel) {
      if (mode == InteractionMode.panning) {
        return InteractionMode.idle;
      }
      return mode;
    }

    return mode;
  }

  InteractionMode _reducePinch({required InteractionMode mode, required InteractionEvent event}) {
    if (event.type == InteractionEventType.pinchStart) {
      if (mode == InteractionMode.dndLocal || mode == InteractionMode.dndExternal) {
        return mode;
      }
      return InteractionMode.pinching;
    }

    if (event.type == InteractionEventType.pinchEnd) {
      if (mode == InteractionMode.pinching) {
        return InteractionMode.idle;
      }
      return mode;
    }

    return mode;
  }

  InteractionMode _reduceSelection({required InteractionMode mode, required InteractionEvent event}) {
    if (event.type == InteractionEventType.selectionHandleDragStart) {
      return InteractionMode.selectionHandleDragging;
    }

    if (event.type == InteractionEventType.selectionHandleDragEnd && mode == InteractionMode.selectionHandleDragging) {
      return InteractionMode.idle;
    }

    if (event.type == InteractionEventType.longPressStart) {
      return InteractionMode.longPressSelecting;
    }

    if (event.type == InteractionEventType.longPressEnd && mode == InteractionMode.longPressSelecting) {
      return InteractionMode.idle;
    }

    if (event.type == InteractionEventType.doubleTapDragStart) {
      return InteractionMode.doubleTapSelecting;
    }

    if (event.type == InteractionEventType.doubleTapDragEnd && mode == InteractionMode.doubleTapSelecting) {
      return InteractionMode.idle;
    }

    return mode;
  }

  InteractionMode _reduceTable({required InteractionMode mode, required InteractionEvent event}) {
    if (event.type == InteractionEventType.tableHandleDragStart) {
      return InteractionMode.tableCellHandleDragging;
    }

    if (event.type == InteractionEventType.tableHandleDragEnd && mode == InteractionMode.tableCellHandleDragging) {
      return InteractionMode.idle;
    }

    return mode;
  }

  ({InteractionMode mode, AuxiliaryGestureKind? kind}) _reduceAuxiliary({
    required InteractionMode mode,
    required AuxiliaryGestureKind? kind,
    required InteractionEvent event,
  }) {
    if (event.type == InteractionEventType.auxiliaryGestureStart) {
      final inputKind = event.kind;
      if (inputKind == null) {
        return (mode: mode, kind: kind);
      }
      return (mode: InteractionMode.auxiliaryGesture, kind: inputKind);
    }

    if (event.type == InteractionEventType.auxiliaryGestureUpdate) {
      if (mode != InteractionMode.auxiliaryGesture) {
        return (mode: mode, kind: kind);
      }
      final inputKind = event.kind;
      if (inputKind == null) {
        return (mode: mode, kind: kind);
      }
      return (mode: mode, kind: inputKind);
    }

    if (event.type == InteractionEventType.auxiliaryGestureEnd && mode == InteractionMode.auxiliaryGesture) {
      return (mode: InteractionMode.idle, kind: null);
    }

    return (mode: mode, kind: kind);
  }

  InteractionMode _reduceDnd({required InteractionMode mode, required InteractionEvent event}) {
    if (event.type == InteractionEventType.dndStart) {
      if (mode == InteractionMode.selectionHandleDragging ||
          mode == InteractionMode.tableCellHandleDragging ||
          mode == InteractionMode.longPressSelecting ||
          mode == InteractionMode.doubleTapSelecting) {
        return mode;
      }

      final local = event.local;
      if (local == null) {
        return mode;
      }

      return local ? InteractionMode.dndLocal : InteractionMode.dndExternal;
    }

    if (event.type == InteractionEventType.dndEnter) {
      if (mode == InteractionMode.dndLocal) {
        return mode;
      }
      return InteractionMode.dndExternal;
    }

    if (event.type == InteractionEventType.dndLeave) {
      if (mode == InteractionMode.dndExternal) {
        return InteractionMode.idle;
      }
      return mode;
    }

    if (event.type == InteractionEventType.dndDrop || event.type == InteractionEventType.dndSessionEnd) {
      return InteractionMode.idle;
    }

    return mode;
  }
}
