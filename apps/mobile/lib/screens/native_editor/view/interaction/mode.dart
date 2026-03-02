enum InteractionMode {
  idle,
  panning,
  pinching,
  auxiliaryGesture,
  textHandleDragging,
  tableCellHandleDragging,
  longPressSelecting,
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
      mode == InteractionMode.textHandleDragging ||
      mode == InteractionMode.tableCellHandleDragging ||
      mode == InteractionMode.longPressSelecting ||
      mode == InteractionMode.doubleTapSelecting;
  bool get isLongPressing => mode == InteractionMode.longPressSelecting;

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
