import 'package:flutter/gestures.dart';

class ConditionalLongPressGestureRecognizer extends LongPressGestureRecognizer {
  ConditionalLongPressGestureRecognizer({required this.condition, super.duration, super.postAcceptSlopTolerance});

  final bool Function(Offset globalPosition) condition;

  @override
  void didExceedDeadline() {
    if (initialPosition == null) {
      super.didExceedDeadline();
      return;
    }

    final globalPosition = initialPosition!.global;
    if (condition(globalPosition)) {
      resolve(GestureDisposition.rejected);
      stopTrackingPointer(primaryPointer!);
    } else {
      super.didExceedDeadline();
    }
  }
}

class ControllerSelectionGestureState {
  Offset? start;
  _ControllerGesturePhase _phase = _ControllerGesturePhase.idle;

  bool get longPressing => _phase == _ControllerGesturePhase.longPress;
  bool get pending => _phase == _ControllerGesturePhase.doubleTapPending;
  bool get dragging => _phase == _ControllerGesturePhase.doubleTapDragging;
  bool get active => pending || dragging;

  bool startLongPress() {
    if (active) {
      return false;
    }
    _setPhase(_ControllerGesturePhase.longPress);
    return true;
  }

  void endLongPress() {
    if (!longPressing) {
      return;
    }
    _setPhase(_ControllerGesturePhase.idle);
  }

  void prepare(Offset startPosition) {
    start = startPosition;
    _setPhase(_ControllerGesturePhase.doubleTapPending);
  }

  void begin(Offset startPosition) {
    start = startPosition;
    _setPhase(_ControllerGesturePhase.doubleTapDragging);
  }

  void clearPending() {
    if (!pending) {
      return;
    }
    _setPhase(_ControllerGesturePhase.idle);
  }

  void stop() {
    if (!active) {
      return;
    }
    _setPhase(_ControllerGesturePhase.idle);
  }

  void _setPhase(_ControllerGesturePhase next) {
    _phase = next;
    if (!pending && !dragging) {
      start = null;
    }
  }
}

enum _ControllerGesturePhase { idle, longPress, doubleTapPending, doubleTapDragging }

class ControllerResumedPanState {
  int? pointer;
  Offset? lastLocalPosition;
  bool active = false;

  void clear() {
    pointer = null;
    lastLocalPosition = null;
    active = false;
  }
}
