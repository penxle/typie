part of '../controller.dart';

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

class LongPressSession implements InteractionSession {
  bool _active = false;

  bool get active => _active;

  bool start() {
    if (_active) {
      return false;
    }
    _active = true;
    return true;
  }

  void end() {
    _active = false;
  }

  @override
  void reset() {
    _active = false;
  }
}

extension LongPressInteractionMethods on EditorInteractionController {
  bool startLongPress(Offset globalPosition) {
    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    if (!_decide(command: InteractionCommand.longPressStart(viewportPosition: viewportPosition))) {
      return false;
    }

    scope.inputController.commitComposing();

    longPressPosition.value = viewportPosition;
    if (!_decide(command: InteractionCommand.longPressBeginSelecting)) {
      return false;
    }
    if (!_longPressSession.start()) {
      _reject(InteractionCommandCategory.longPress, InteractionBlockReason.sessionAlreadyActive);
      return false;
    }

    final next = _applyTransition(InteractionEvent.longPressStart);
    if (next.mode != InteractionMode.longPressSelecting) {
      _longPressSession.end();
      _reject(InteractionCommandCategory.longPress, InteractionBlockReason.modeRejected);
      return false;
    }
    _setBlockReason(InteractionCommandCategory.longPress, null);

    final state = scope.controller.state;
    final draggingHandle = state.draggingHandle;
    final anchorHandle = draggingHandle == SelectionHandleType.from
        ? state.selection?.toBounds
        : state.selection?.fromBounds;

    _handleDragSession.beginLongPressSession(
      touchPosition: globalPosition,
      handleScreenPosition: _handleStemCenter(state.selection?.fromBounds ?? state.selection?.toBounds, readGeometry()),
      anchorHandle: anchorHandle,
    );
    return true;
  }

  bool updateLongPress(Offset viewportPosition) {
    if (!_decide(command: InteractionCommand.longPressUpdate)) {
      return false;
    }

    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    longPressPosition.value = viewportPosition;

    if (pageIdx >= 0) {
      final pointerX = _resolvePointerX(viewportPosition.dx);
      scope.controller.dispatch({
        'type': 'pointerDown',
        'pageIdx': pageIdx,
        'x': pointerX,
        'y': localY,
        'clickCount': 1,
        'button': 'primary',
        'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
      });
      scope.controller.dispatch({
        'type': 'pointerUp',
        'pageIdx': pageIdx,
        'x': pointerX,
        'y': localY,
        'button': 'primary',
        'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
      });
      scope.controller.scrollIntoView();
    }

    _handleAutoScroll(y: viewportPosition.dy, x: viewportPosition.dx);
    return true;
  }

  bool endLongPress() {
    if (!_decide(command: InteractionCommand.longPressEnd)) {
      return false;
    }

    longPressPosition.value = null;
    _autoScrollSession.stop();
    _longPressSession.end();
    _applyTransition(InteractionEvent.longPressEnd);
    return true;
  }
}
