import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/view/interaction/input.dart';
import 'package:typie/screens/native_editor/view/interaction/mode.dart';

class EditorInteractionState {
  final ValueNotifier<InteractionSnapshot> _snapshotNotifier = ValueNotifier(const InteractionSnapshot());
  bool _disposed = false;
  final List<InteractionInput> _deferredInputs = [];
  bool _deferredFlushScheduled = false;

  ValueListenable<InteractionSnapshot> get listenable => _snapshotNotifier;

  InteractionSnapshot snapshot() => _snapshotNotifier.value;

  void reset() {
    _deferredInputs.clear();
    _deferredFlushScheduled = false;
    _snapshotNotifier.value = const InteractionSnapshot();
  }

  void dispose() {
    _disposed = true;
    _deferredInputs.clear();
    _deferredFlushScheduled = false;
    _snapshotNotifier.dispose();
  }

  void handle(InteractionInput input) {
    if (_shouldDeferMutation()) {
      _deferredInputs.add(input);
      _scheduleDeferredFlush();
      return;
    }
    _applyInput(input);
  }

  void _applyInput(InteractionInput input) {
    final previous = _snapshotNotifier.value;

    var nextMode = previous.mode;
    var nextAuxiliaryKind = previous.auxiliaryGestureKind;

    if (input is PointerCancelInput) {
      nextMode = InteractionMode.idle;
      nextAuxiliaryKind = null;
    }

    final dndMode = _handleDndMode(currentMode: nextMode, input: input);
    if (dndMode != nextMode) {
      nextMode = dndMode;
      if (nextMode != InteractionMode.auxiliaryGesture) {
        nextAuxiliaryKind = null;
      }
    }

    if (!nextMode.isDndActive) {
      nextMode = _handlePinchMode(currentMode: nextMode, input: input);

      if (nextMode == InteractionMode.pinching) {
        nextAuxiliaryKind = null;
      }

      if (nextMode != InteractionMode.pinching) {
        final auxiliary = _handleAuxiliaryGestureMode(
          currentMode: nextMode,
          currentKind: nextAuxiliaryKind,
          input: input,
        );
        nextMode = auxiliary.mode;
        nextAuxiliaryKind = auxiliary.kind;

        nextMode = _handleTableMode(currentMode: nextMode, input: input);
        nextMode = _handleSelectionMode(currentMode: nextMode, input: input);
        nextMode = _handlePanMode(currentMode: nextMode, input: input);
      }
    }

    final nextSnapshot = previous.copyWith(
      mode: nextMode,
      auxiliaryGestureKind: nextAuxiliaryKind,
      clearAuxiliaryGestureKind: nextMode != InteractionMode.auxiliaryGesture,
    );

    if (!_equalsSnapshot(previous, nextSnapshot)) {
      _snapshotNotifier.value = nextSnapshot;
    }
  }

  bool _shouldDeferMutation() {
    if (_disposed) {
      return false;
    }
    SchedulerPhase? phase;
    try {
      phase = SchedulerBinding.instance.schedulerPhase;
    } catch (_) {
      return false;
    }
    return phase == SchedulerPhase.persistentCallbacks;
  }

  void _scheduleDeferredFlush() {
    if (_deferredFlushScheduled || _disposed) {
      return;
    }
    _deferredFlushScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _deferredFlushScheduled = false;
      if (_disposed || _deferredInputs.isEmpty) {
        _deferredInputs.clear();
        return;
      }

      final pending = List<InteractionInput>.from(_deferredInputs);
      _deferredInputs.clear();
      for (final input in pending) {
        if (_disposed) {
          break;
        }
        _applyInput(input);
      }
    });
  }

  bool _equalsSnapshot(InteractionSnapshot a, InteractionSnapshot b) {
    return a.mode == b.mode && a.auxiliaryGestureKind == b.auxiliaryGestureKind;
  }

  InteractionMode _handlePanMode({required InteractionMode currentMode, required InteractionInput input}) {
    if (input is PanStartInput) {
      if (currentMode == InteractionMode.idle) {
        return InteractionMode.panning;
      }
      return currentMode;
    }

    if (input is PanEndInput || input is PanCancelInput) {
      if (currentMode == InteractionMode.panning) {
        return InteractionMode.idle;
      }
      return currentMode;
    }

    return currentMode;
  }

  InteractionMode _handlePinchMode({required InteractionMode currentMode, required InteractionInput input}) {
    if (input is PinchStartInput) {
      if (currentMode == InteractionMode.dndLocal || currentMode == InteractionMode.dndExternal) {
        return currentMode;
      }
      return InteractionMode.pinching;
    }

    if (input is PinchEndInput) {
      if (currentMode == InteractionMode.pinching) {
        return InteractionMode.idle;
      }
      return currentMode;
    }

    return currentMode;
  }

  InteractionMode _handleSelectionMode({required InteractionMode currentMode, required InteractionInput input}) {
    if (input is TextHandleDragStartInput) {
      return InteractionMode.textHandleDragging;
    }

    if (input is TextHandleDragEndInput && currentMode == InteractionMode.textHandleDragging) {
      return InteractionMode.idle;
    }

    if (input is LongPressStartInput) {
      return InteractionMode.longPressSelecting;
    }

    if (input is LongPressEndInput && currentMode == InteractionMode.longPressSelecting) {
      return InteractionMode.idle;
    }

    if (input is DoubleTapDragStartInput) {
      return InteractionMode.doubleTapSelecting;
    }

    if (input is DoubleTapDragEndInput && currentMode == InteractionMode.doubleTapSelecting) {
      return InteractionMode.idle;
    }

    return currentMode;
  }

  InteractionMode _handleTableMode({required InteractionMode currentMode, required InteractionInput input}) {
    if (input is TableHandleDragStartInput) {
      return InteractionMode.tableCellHandleDragging;
    }

    if (input is TableHandleDragEndInput && currentMode == InteractionMode.tableCellHandleDragging) {
      return InteractionMode.idle;
    }

    return currentMode;
  }

  ({InteractionMode mode, AuxiliaryGestureKind? kind}) _handleAuxiliaryGestureMode({
    required InteractionMode currentMode,
    required AuxiliaryGestureKind? currentKind,
    required InteractionInput input,
  }) {
    if (input is AuxiliaryGestureStartInput) {
      return (mode: InteractionMode.auxiliaryGesture, kind: input.kind);
    }

    if (input is AuxiliaryGestureUpdateInput) {
      if (currentMode != InteractionMode.auxiliaryGesture) {
        return (mode: currentMode, kind: currentKind);
      }
      return (mode: currentMode, kind: input.kind);
    }

    if (input is AuxiliaryGestureEndInput && currentMode == InteractionMode.auxiliaryGesture) {
      return (mode: InteractionMode.idle, kind: null);
    }

    return (mode: currentMode, kind: currentKind);
  }

  InteractionMode _handleDndMode({required InteractionMode currentMode, required InteractionInput input}) {
    if (input is DndStartInput) {
      return input.local ? InteractionMode.dndLocal : InteractionMode.dndExternal;
    }

    if (input is DndEnterInput) {
      if (currentMode == InteractionMode.dndLocal) {
        return currentMode;
      }
      return InteractionMode.dndExternal;
    }

    if (input is DndLeaveInput) {
      if (currentMode == InteractionMode.dndExternal) {
        return InteractionMode.idle;
      }
      return currentMode;
    }

    if (input is DndDropInput || input is DndSessionEndInput) {
      return InteractionMode.idle;
    }

    return currentMode;
  }
}

extension on InteractionMode {
  bool get isDndActive => this == InteractionMode.dndLocal || this == InteractionMode.dndExternal;
}
