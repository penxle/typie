part of '../controller.dart';

class PanSession implements InteractionSession {
  Drag? _verticalDrag;
  Drag? _horizontalDrag;
  bool _horizontalPanEnabled = false;

  bool get hasScrollDrag => _verticalDrag != null || _horizontalDrag != null;

  void startDrag({
    required DragStartDetails details,
    required bool allowHorizontal,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics horizontalMetrics,
  }) {
    final horizontalPosition = horizontalMetrics.activePosition;
    final canStartHorizontal = allowHorizontal && horizontalMetrics.canScrollHorizontally && horizontalPosition != null;

    _horizontalPanEnabled = canStartHorizontal;
    final verticalPosition = resolveScrollPosition(verticalScrollController);
    if (verticalPosition != null) {
      _verticalDrag = verticalPosition.drag(details, () {
        _verticalDrag = null;
      });
    }

    if (canStartHorizontal) {
      _horizontalDrag = horizontalPosition.drag(details, () {
        _horizontalDrag = null;
      });
    }
  }

  void updateDrag(DragUpdateDetails details, {required HorizontalScrollMetrics horizontalMetrics}) {
    final horizontalPosition = horizontalMetrics.activePosition;
    final canFallbackHorizontal =
        _horizontalPanEnabled &&
        horizontalMetrics.canScrollHorizontally &&
        horizontalPosition != null &&
        details.delta.dx != 0;
    final horizontalBefore = canFallbackHorizontal ? horizontalPosition.pixels : null;

    _verticalDrag?.update(
      DragUpdateDetails(
        globalPosition: details.globalPosition,
        delta: Offset(0, details.delta.dy),
        primaryDelta: details.delta.dy,
        sourceTimeStamp: details.sourceTimeStamp,
      ),
    );

    _horizontalDrag?.update(
      DragUpdateDetails(
        globalPosition: details.globalPosition,
        delta: Offset(details.delta.dx, 0),
        primaryDelta: details.delta.dx,
        sourceTimeStamp: details.sourceTimeStamp,
      ),
    );

    if (canFallbackHorizontal) {
      final horizontalAfterDrag = horizontalPosition.pixels;
      final dragMoved = horizontalBefore != null && (horizontalAfterDrag - horizontalBefore).abs() > 0.01;
      if (!dragMoved) {
        final nextOffset = (horizontalAfterDrag - details.delta.dx).clamp(0.0, horizontalPosition.maxScrollExtent);
        if ((nextOffset - horizontalAfterDrag).abs() > 0) {
          horizontalPosition.jumpTo(nextOffset);
        }
      }
    }
  }

  void applyRawPanDelta({
    required Offset delta,
    required bool allowHorizontal,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics horizontalMetrics,
  }) {
    final verticalPosition = resolveScrollPosition(verticalScrollController);
    if (verticalPosition != null &&
        verticalPosition.hasContentDimensions &&
        verticalPosition.maxScrollExtent > 0 &&
        delta.dy != 0) {
      final currentOffset = verticalPosition.pixels;
      final nextOffset = (currentOffset - delta.dy).clamp(0.0, verticalPosition.maxScrollExtent);
      if ((nextOffset - currentOffset).abs() > 0) {
        verticalPosition.jumpTo(nextOffset);
      }
    }

    final horizontalPosition = horizontalMetrics.activePosition;
    if (allowHorizontal && horizontalMetrics.canScrollHorizontally && horizontalPosition != null && delta.dx != 0) {
      final currentOffset = horizontalPosition.pixels;
      final nextOffset = (currentOffset - delta.dx).clamp(0.0, horizontalPosition.maxScrollExtent);
      if ((nextOffset - currentOffset).abs() > 0) {
        horizontalPosition.jumpTo(nextOffset);
      }
    }
  }

  void endDrag(DragEndDetails details) {
    _verticalDrag?.end(
      DragEndDetails(
        velocity: Velocity(pixelsPerSecond: Offset(0, details.velocity.pixelsPerSecond.dy)),
        primaryVelocity: details.velocity.pixelsPerSecond.dy,
      ),
    );

    _horizontalDrag?.end(
      DragEndDetails(
        velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
        primaryVelocity: details.velocity.pixelsPerSecond.dx,
      ),
    );

    _verticalDrag = null;
    _horizontalDrag = null;
    _horizontalPanEnabled = false;
  }

  void cancelDrag() {
    _verticalDrag?.cancel();
    _horizontalDrag?.cancel();
    _verticalDrag = null;
    _horizontalDrag = null;
    _horizontalPanEnabled = false;
  }

  @override
  void reset() {
    cancelDrag();
  }
}

class PanResumeSession implements InteractionSession {
  int? pointer;
  Offset? lastLocalPosition;
  bool active = false;

  @override
  void reset() {
    pointer = null;
    lastLocalPosition = null;
    active = false;
  }
}

extension PanInteractionMethods on EditorInteractionController {
  void onPanStart(DragStartDetails details) {
    final dndLocked = _consumeIfDndLocked(recover: true, onLocked: _panSession.cancelDrag);
    if (dndLocked) {
      _reject(InteractionCommandCategory.pan, InteractionBlockReason.dndLocked);
      return;
    }
    if (!_decide(command: InteractionCommand.panStart)) {
      return;
    }

    final next = _applyTransition(InteractionEvent.panStart);
    if (next.mode != InteractionMode.panning) {
      _reject(InteractionCommandCategory.pan, InteractionBlockReason.modeRejected);
      return;
    }
    _setBlockReason(InteractionCommandCategory.pan, null);

    _panSession.startDrag(
      details: details,
      allowHorizontal: _allowHorizontalPan,
      verticalScrollController: scope.verticalScrollController,
      horizontalMetrics: _resolveHorizontalMetrics(),
    );
  }

  void onPanUpdate(DragUpdateDetails details) {
    final dndLocked = _consumeIfDndLocked(onLocked: _panSession.cancelDrag);
    if (dndLocked) {
      _reject(InteractionCommandCategory.pan, InteractionBlockReason.dndLocked);
      return;
    }
    if (!_decide(command: InteractionCommand.panUpdate)) {
      return;
    }

    _panSession.updateDrag(details, horizontalMetrics: _resolveHorizontalMetrics());
  }

  void onPanEnd(DragEndDetails details) {
    final dndLocked = _consumeIfDndLocked(onLocked: _panSession.cancelDrag);
    if (dndLocked) {
      _reject(InteractionCommandCategory.pan, InteractionBlockReason.dndLocked);
      return;
    }
    if (!_decide(command: InteractionCommand.panEnd)) {
      return;
    }

    _applyTransition(InteractionEvent.panEnd);
    _panSession.endDrag(details);
  }

  void onPanCancel() {
    final dndLocked = _consumeIfDndLocked(onLocked: _panSession.cancelDrag);
    if (dndLocked) {
      _reject(InteractionCommandCategory.pan, InteractionBlockReason.dndLocked);
      return;
    }
    if (!_decide(command: InteractionCommand.panCancel)) {
      return;
    }

    _applyTransition(InteractionEvent.panCancel);
    _panSession.cancelDrag();
  }

  void onPointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || pinchSession.isPinching) {
      return;
    }

    if (_consumeIfDndLocked(recover: true)) {
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

    _panSession.applyRawPanDelta(
      delta: Offset(-scrollDx, -scrollDy),
      allowHorizontal: _allowHorizontalPan,
      verticalScrollController: scope.verticalScrollController,
      horizontalMetrics: _resolveHorizontalMetrics(),
    );
  }

  void onPointerDown(PointerDownEvent event) {
    _applyTransition(InteractionEvent.pointerDown);
    if (_consumeIfDndLocked(recover: true)) {
      return;
    }

    _clearResumedPanState();

    pinchSession.addPointer(event.pointer, event.localPosition);
    if (pinchSession.pointerCount >= 2) {
      _beginPinchIfNeeded();
    }
  }

  bool _handleSelectionPointerMove(PointerMoveEvent event) {
    final localPosition = event.localPosition;
    if (_doubleTapDragSession.pending) {
      final startPosition = _doubleTapDragSession.start;
      if (startPosition != null && (localPosition - startPosition).distance >= 4) {
        if (startDoubleTapDrag(startPosition)) {
          updateDoubleTapDragSelection(localPosition);
        }
      }
      return true;
    }

    if (_doubleTapDragSession.dragging) {
      updateDoubleTapDragSelection(localPosition);
      return true;
    }

    if (_longPressSession.active) {
      updateLongPress(localPosition);
      return true;
    }

    return false;
  }

  bool _handleResumedPanPointerMove(PointerMoveEvent event) {
    if (_panResumeSession.pointer != event.pointer) {
      return false;
    }

    final previous = _panResumeSession.lastLocalPosition;
    _panResumeSession.lastLocalPosition = event.localPosition;
    if (previous == null) {
      return true;
    }

    if (!_decide(command: InteractionCommand.panResume)) {
      _clearResumedPanState();
      return true;
    }

    final delta = event.localPosition - previous;
    if (!_panResumeSession.active) {
      if (delta.distance < 1) {
        return true;
      }
      _panSession.startDrag(
        details: DragStartDetails(globalPosition: event.position, localPosition: previous),
        allowHorizontal: _allowHorizontalPan,
        verticalScrollController: scope.verticalScrollController,
        horizontalMetrics: _resolveHorizontalMetrics(),
      );
      _panResumeSession.active = true;
    }

    if (delta.distance > 0) {
      _panSession.updateDrag(
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

    final previousPointerPosition = pinchSession.pointerPosition(event.pointer);
    if (pinchSession.containsPointer(event.pointer)) {
      pinchSession.updatePointer(event.pointer, event.localPosition);
    }

    if (pinchSession.isPinching) {
      _updatePinchZoom();
      return;
    }

    if (_consumeIfDndLocked(
      onLocked: () {
        if (_panSession.hasScrollDrag) {
          _panSession.cancelDrag();
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
      _panSession.applyRawPanDelta(
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

    if (_panResumeSession.pointer == event.pointer) {
      if (_panResumeSession.active) {
        if (canceled) {
          _panSession.cancelDrag();
        } else {
          _panSession.endDrag(DragEndDetails());
        }
      }
      _clearResumedPanState();
    }

    final wasPinching = pinchSession.isPinching;
    pinchSession.removePointer(event.pointer);
    if (pinchSession.pointerCount < 2) {
      _endPinchIfNeeded();
    }

    if (wasPinching && pinchSession.pointerCount == 1) {
      final remaining = pinchSession.singlePointerEntry;
      if (remaining != null) {
        _panResumeSession.pointer = remaining.key;
        _panResumeSession.lastLocalPosition = remaining.value;
        _panResumeSession.active = false;
      }
    }

    if (wasPinching) {
      return;
    }

    if (_doubleTapDragSession.dragging) {
      endDoubleTapDrag();
      return;
    }

    final hadPendingDoubleTap = _doubleTapDragSession.pending;
    _doubleTapDragSession.clearPending();
    if (!canceled && hadPendingDoubleTap) {
      final hasRangeSelection = !(scope.controller.state.selection?.collapsed ?? true);
      if (hasRangeSelection) {
        showContextMenu.value = true;
      }
    }

    endLongPress();
    _endSelectionHandleDrag();
  }
}
