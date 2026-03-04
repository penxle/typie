import 'package:flutter/foundation.dart';

import 'core.dart';

enum InteractionMode {
  idle,
  panning,
  pinching,
  auxiliaryGesture,
  selectionHandleDragging,
  tableCellHandleDragging,
  longPressSelecting,
  longPressWordSelecting,
  doubleTapSelecting,
  dndLocal,
  dndExternal,
}

enum AuxiliaryGestureKind { imageResize, tableColumnResize }

class InteractionSnapshot {
  const InteractionSnapshot({this.mode = InteractionMode.idle, this.auxiliaryGestureKind});

  final InteractionMode mode;
  final AuxiliaryGestureKind? auxiliaryGestureKind;

  bool get isDndActive => mode == InteractionMode.dndLocal || mode == InteractionMode.dndExternal;
  bool get isPinching => mode == InteractionMode.pinching;
  bool get isAuxiliaryGesture => mode == InteractionMode.auxiliaryGesture;
  bool get isSelecting =>
      mode == InteractionMode.selectionHandleDragging ||
      mode == InteractionMode.tableCellHandleDragging ||
      mode == InteractionMode.longPressSelecting ||
      mode == InteractionMode.longPressWordSelecting ||
      mode == InteractionMode.doubleTapSelecting;
  bool get isLongPressing =>
      mode == InteractionMode.longPressSelecting || mode == InteractionMode.longPressWordSelecting;

  InteractionSnapshot copyWith({
    InteractionMode? mode,
    AuxiliaryGestureKind? auxiliaryGestureKind,
    bool clearAuxiliaryGestureKind = false,
  }) {
    return InteractionSnapshot(
      mode: mode ?? this.mode,
      auxiliaryGestureKind: clearAuxiliaryGestureKind ? null : (auxiliaryGestureKind ?? this.auxiliaryGestureKind),
    );
  }
}

class EditorInteractionState {
  final ValueNotifier<InteractionSnapshot> _snapshotNotifier = ValueNotifier(const InteractionSnapshot());
  static const _core = InteractionCore();

  ValueListenable<InteractionSnapshot> get listenable => _snapshotNotifier;

  InteractionSnapshot snapshot() => _snapshotNotifier.value;

  void reset() {
    _snapshotNotifier.value = const InteractionSnapshot();
  }

  void dispose() {
    _snapshotNotifier.dispose();
  }

  void handle(InteractionEvent event) {
    final previous = _snapshotNotifier.value;
    final nextSnapshot = _core.reduce(previous: previous, event: event);

    if (!_equalsSnapshot(previous, nextSnapshot)) {
      _snapshotNotifier.value = nextSnapshot;
    }
  }

  bool _equalsSnapshot(InteractionSnapshot a, InteractionSnapshot b) {
    return a.mode == b.mode && a.auxiliaryGestureKind == b.auxiliaryGestureKind;
  }
}
