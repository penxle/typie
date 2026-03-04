part of '../controller.dart';

extension PointerGestureMethods on EditorInteractionController {
  bool _beginPanModeForRawInput() {
    final mode = scope.interactionState.snapshot().mode;
    if (mode == InteractionMode.panning) {
      return true;
    }
    if (mode != InteractionMode.idle) {
      return false;
    }
    if (!_decide(command: InteractionCommand.panStart)) {
      return false;
    }

    final next = _applyTransition(InteractionEvent.panStart);
    return next.mode == InteractionMode.panning;
  }

  void _endPanModeForRawInput() {
    if (_panGesture.hasScrollDrag) {
      return;
    }
    if (scope.interactionState.snapshot().mode == InteractionMode.panning) {
      _applyTransition(InteractionEvent.panEnd);
    }
  }

  void onPointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || pinchGesture.isPinching) {
      return;
    }

    if (!_decide(
      command: const InteractionCommand.panApplyRaw(hasPreviousPointerPosition: true, fromPointerSignal: true),
    )) {
      return;
    }

    if (_consumeIfDndLocked()) {
      return;
    }

    final keysPressed = HardwareKeyboard.instance.logicalKeysPressed;
    if (_handlePointerZoom(event, keysPressed)) {
      return;
    }

    final isShiftPressed =
        keysPressed.contains(LogicalKeyboardKey.shiftLeft) || keysPressed.contains(LogicalKeyboardKey.shiftRight);

    var scrollDx = event.scrollDelta.dx;
    var scrollDy = event.scrollDelta.dy;

    if (_allowHorizontalPan && isShiftPressed && scrollDx == 0 && scrollDy != 0) {
      scrollDx = scrollDy;
      scrollDy = 0;
    }

    if (scrollDx == 0 && scrollDy == 0) {
      return;
    }

    if (!_beginPanModeForRawInput()) {
      return;
    }

    _panGesture.applyRawPanDelta(
      delta: Offset(-scrollDx, -scrollDy),
      allowHorizontal: _allowHorizontalPan,
      verticalScrollController: scope.verticalScrollController,
      horizontalMetrics: _resolveHorizontalMetrics(),
    );
    _endPanModeForRawInput();
  }

  void onPointerDown(PointerDownEvent event) {
    _applyTransition(InteractionEvent.pointerDown);
    if (_consumeIfDndLocked()) {
      return;
    }

    _clearResumedPanState();
    primeLongPressModeAtPointerDown(event.localPosition);

    pinchGesture.addPointer(event.pointer, event.localPosition);
    if (pinchGesture.pointerCount >= 2) {
      _beginPinchIfNeeded();
    }
  }

  bool _handleSelectionPointerMove(PointerMoveEvent event) {
    final localPosition = event.localPosition;
    if (_doubleTapDragGesture.pending) {
      final startPosition = _doubleTapDragGesture.start;
      if (startPosition != null && (localPosition - startPosition).distance >= 4) {
        if (startDoubleTapDrag(startPosition)) {
          updateDoubleTapDragSelection(localPosition);
        }
      }
      return true;
    }

    if (_doubleTapDragGesture.dragging) {
      updateDoubleTapDragSelection(localPosition);
      return true;
    }

    if (scope.interactionState.snapshot().isLongPressing) {
      updateLongPress(localPosition);
      return true;
    }

    return false;
  }

  bool _handleResumedPanPointerMove(PointerMoveEvent event) {
    if (_panResumeGesture.pointer != event.pointer) {
      return false;
    }

    final previous = _panResumeGesture.lastLocalPosition;
    _panResumeGesture.lastLocalPosition = event.localPosition;
    if (previous == null) {
      return true;
    }

    if (!_decide(command: InteractionCommand.panResume)) {
      _clearResumedPanState();
      return true;
    }

    final delta = event.localPosition - previous;
    if (!_panGesture.hasScrollDrag) {
      if (delta.distance < 1) {
        return true;
      }
      if (!_beginPanModeForRawInput()) {
        _clearResumedPanState();
        return true;
      }
      _panGesture.startDrag(
        details: DragStartDetails(globalPosition: event.position, localPosition: previous),
        allowHorizontal: _allowHorizontalPan,
        verticalScrollController: scope.verticalScrollController,
        horizontalMetrics: _resolveHorizontalMetrics(),
      );
    }

    if (delta.distance > 0) {
      _panGesture.updateDrag(
        DragUpdateDetails(
          globalPosition: event.position,
          localPosition: event.localPosition,
          delta: delta,
          sourceTimeStamp: event.timeStamp,
        ),
        horizontalMetrics: _resolveHorizontalMetrics(),
      );
    }

    return true;
  }

  void onPointerMove(PointerMoveEvent event) {
    _applyTransition(InteractionEvent.pointerMove);

    final previousPointerPosition = pinchGesture.pointerPosition(event.pointer);
    if (pinchGesture.containsPointer(event.pointer)) {
      pinchGesture.updatePointer(event.pointer, event.localPosition);
    }

    if (pinchGesture.isPinching) {
      _updatePinchZoom();
      return;
    }

    if (_consumeIfDndLocked(
      onLocked: () {
        if (_panGesture.hasScrollDrag) {
          _panGesture.cancelDrag();
        }
        _clearResumedPanState();
      },
    )) {
      return;
    }

    if (_handleResumedPanPointerMove(event)) {
      return;
    }

    if (_handleSelectionPointerMove(event)) {
      return;
    }

    if (!_decide(
      command: InteractionCommand.panApplyRaw(hasPreviousPointerPosition: previousPointerPosition != null),
    )) {
      return;
    }

    final trackedPreviousPointerPosition = previousPointerPosition;
    if (trackedPreviousPointerPosition == null) {
      return;
    }

    final delta = event.localPosition - trackedPreviousPointerPosition;
    if (delta.distance >= 1) {
      if (!_beginPanModeForRawInput()) {
        return;
      }
      _panGesture.applyRawPanDelta(
        delta: delta,
        allowHorizontal: _allowHorizontalPan,
        verticalScrollController: scope.verticalScrollController,
        horizontalMetrics: _resolveHorizontalMetrics(),
      );
    }
  }

  void onPointerUp(PointerUpEvent event) {
    _handlePointerRelease(event, canceled: false);
  }

  void onPointerCancel(PointerCancelEvent event) {
    _handlePointerRelease(event, canceled: true);
  }

  void _handlePointerRelease(PointerEvent event, {required bool canceled}) {
    if (canceled) {
      _applyTransition(InteractionEvent.pointerCancel);
    } else {
      _applyTransition(InteractionEvent.pointerUp);
    }

    if (_panResumeGesture.pointer == event.pointer) {
      if (_panGesture.hasScrollDrag) {
        if (canceled) {
          _panGesture.cancelDrag();
        } else {
          _panGesture.endDrag(DragEndDetails());
        }
        _endPanModeForRawInput();
      }
      _clearResumedPanState();
    }

    final wasPinching = pinchGesture.isPinching;
    pinchGesture.removePointer(event.pointer);
    if (pinchGesture.pointerCount < 2) {
      _endPinchIfNeeded();
    }

    if (wasPinching && pinchGesture.pointerCount == 1) {
      final remaining = pinchGesture.singlePointerEntry;
      if (remaining != null) {
        _panResumeGesture.pointer = remaining.key;
        _panResumeGesture.lastLocalPosition = remaining.value;
      }
    }

    if (wasPinching) {
      return;
    }

    if (_doubleTapDragGesture.dragging) {
      endDoubleTapDrag();
      return;
    }

    final hadPendingDoubleTap = _doubleTapDragGesture.pending;
    _doubleTapDragGesture.clearPending();
    if (!canceled && hadPendingDoubleTap) {
      final hasRangeSelection = !(scope.controller.state.selection?.collapsed ?? true);
      if (hasRangeSelection) {
        showContextMenu.value = true;
      }
    }

    _endPanModeForRawInput();
    endLongPress();
    _endSelectionHandleDrag();
  }
}
