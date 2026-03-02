part of 'controller.dart';

extension ControllerPointerMethods on EditorInteractionController {
  String? _zoomSnapKey(double value) {
    final layout = scope.controller.state.layout;
    final viewWidth = readViewWidth();
    if (layout is! PaginatedLayout || viewWidth <= 0) {
      return null;
    }

    final fitWidthZoom = computePaginatedFitWidthZoom(pageWidth: layout.pageWidth, viewportWidth: viewWidth);
    final unitZoom = clampDocumentZoom(1, bounds: computePaginatedZoomBounds(pageWidth: layout.pageWidth));

    if (zoomEquals(value, fitWidthZoom)) {
      return 'fit-width';
    }
    if (zoomEquals(value, unitZoom)) {
      return 'unit';
    }
    return null;
  }

  void _maybeSendZoomSnapHaptic({required double previousZoom, required double nextZoom}) {
    if (zoomEquals(previousZoom, nextZoom)) {
      return;
    }

    final nextSnap = _zoomSnapKey(nextZoom);
    if (nextSnap == null) {
      return;
    }

    final previousSnap = _zoomSnapKey(previousZoom);
    if (previousSnap == nextSnap) {
      return;
    }

    unawaited(HapticFeedback.selectionClick());
  }

  void _beginPinchIfNeeded() {
    final geometry = readGeometry();
    final started = pinch.beginIfNeeded(
      isPaginated: geometry.isPaginated,
      currentZoom: scope.displayZoom.value,
      resolveLogicalX: gesture.getPointerX,
      resolvePageAtPosition: getPageAtPosition,
    );
    if (!started) {
      return;
    }

    _handleInteractionInput(const PinchStartInput());

    _clearResumedPanState();

    gesture
      ..cancelTapTimer()
      ..stopSelectionHandlesAndAutoScroll()
      ..cancelScrollDrag();
    _gestureState.stop();
    longPressPosition.value = null;
    handleDragPosition.value = null;
    showContextMenu.value = false;
  }

  void _updatePinchZoom() {
    final geometry = readGeometry();
    pinch.updateIfNeeded(
      isPaginated: geometry.isPaginated,
      layout: scope.controller.state.layout,
      viewportWidth: readViewWidth(),
      currentZoom: scope.displayZoom.value,
      resolveLogicalX: gesture.getPointerX,
      resolvePageAtPosition: getPageAtPosition,
      setZoom: scope.setZoom,
      geometryBuilder: (nextZoom) => ContentGeometry(
        layout: scope.controller.state.layout!,
        pages: scope.controller.state.pages,
        titleAreaHeight: scope.titleAreaHeight.value,
        selection: scope.controller.state.selection,
        zoom: nextZoom,
      ),
      horizontalScrollController: scope.horizontalScrollController,
      verticalScrollController: scope.verticalScrollController,
      isMounted: () => context.mounted,
      onZoomChanged: (previousZoom, nextZoom) {
        _maybeSendZoomSnapHaptic(previousZoom: previousZoom, nextZoom: nextZoom);
      },
    );
  }

  void _endPinchIfNeeded() {
    pinch.endIfNeeded(currentZoom: scope.displayZoom.value, setZoom: scope.setZoom);
    _handleInteractionInput(const PinchEndInput());
  }

  void onPanStart(DragStartDetails details) {
    if (pinch.isPinching) {
      return;
    }
    if (_consumeIfDndLocked(recover: true, onLocked: gesture.cancelScrollDrag)) {
      return;
    }
    if (_gestureState.active) {
      return;
    }

    _handleInteractionInput(const PanStartInput());
    if (scope.interactionState.snapshot().mode != InteractionMode.panning) {
      return;
    }

    gesture.startScrollDrag(details: details, allowHorizontal: _allowHorizontalPan);
  }

  void onPanUpdate(DragUpdateDetails details) {
    if (pinch.isPinching) {
      return;
    }
    if (_consumeIfDndLocked(onLocked: gesture.cancelScrollDrag)) {
      return;
    }

    gesture.updateScrollDrag(details);
  }

  void onPanEnd(DragEndDetails details) {
    if (pinch.isPinching) {
      return;
    }
    if (_consumeIfDndLocked(onLocked: gesture.cancelScrollDrag)) {
      return;
    }
    if (_gestureState.active) {
      return;
    }

    _handleInteractionInput(const PanEndInput());
    gesture.endScrollDrag(details);
  }

  void onPanCancel() {
    if (pinch.isPinching) {
      return;
    }
    if (_consumeIfDndLocked(onLocked: gesture.cancelScrollDrag)) {
      return;
    }
    if (_gestureState.active) {
      return;
    }

    _handleInteractionInput(const PanCancelInput());
    gesture.cancelScrollDrag();
  }

  bool _handlePointerZoom(PointerScrollEvent event, Set<LogicalKeyboardKey> keysPressed) {
    final geometry = readGeometry();
    if (!geometry.isPaginated) {
      return false;
    }

    final isZoomModifierPressed =
        keysPressed.contains(LogicalKeyboardKey.controlLeft) ||
        keysPressed.contains(LogicalKeyboardKey.controlRight) ||
        keysPressed.contains(LogicalKeyboardKey.metaLeft) ||
        keysPressed.contains(LogicalKeyboardKey.metaRight);
    if (!isZoomModifierPressed) {
      return false;
    }

    final layout = scope.controller.state.layout;
    if (layout is! PaginatedLayout) {
      return false;
    }

    final zoomDelta = event.scrollDelta.dy.abs() >= event.scrollDelta.dx.abs()
        ? event.scrollDelta.dy
        : event.scrollDelta.dx;
    if (zoomDelta == 0) {
      return true;
    }

    final currentZoom = scope.displayZoom.value;
    final nextZoom = clampPaginatedZoom(
      zoom: currentZoom * math.exp(-zoomDelta / 240),
      pageWidth: layout.pageWidth,
      viewportWidth: readViewWidth(),
    );
    if (zoomEquals(nextZoom, currentZoom)) {
      return true;
    }

    final focal = event.localPosition;
    final logicalX = gesture.getPointerX(focal.dx);
    final (pageIdx, logicalY) = getPageAtPosition(focal.dy);
    wheelZoomSession.captureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);

    _maybeSendZoomSnapHaptic(previousZoom: currentZoom, nextZoom: nextZoom);
    scope.setZoom(nextZoom);

    wheelZoomSession.syncViewport(
      focal: focal,
      geometry: ContentGeometry(
        layout: layout,
        pages: scope.controller.state.pages,
        titleAreaHeight: scope.titleAreaHeight.value,
        selection: scope.controller.state.selection,
        zoom: nextZoom,
      ),
      viewportWidth: readViewWidth(),
      horizontalScrollController: scope.horizontalScrollController,
      verticalScrollController: scope.verticalScrollController,
      isMounted: () => context.mounted,
      isPinching: () => true,
    );
    return true;
  }

  void onPointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || pinch.isPinching) {
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

    gesture.applyRawPanDelta(delta: Offset(-scrollDx, -scrollDy), allowHorizontal: _allowHorizontalPan);
  }

  void onPointerDown(PointerDownEvent event) {
    _handleInteractionInput(PointerDownInput(pointer: event.pointer));
    if (_consumeIfDndLocked(recover: true)) {
      return;
    }

    _clearResumedPanState();

    pinch.addPointer(event.pointer, event.localPosition);
    if (pinch.pointerCount >= 2) {
      _beginPinchIfNeeded();
    }
  }

  bool _handleSelectionPointerMove(Offset localPosition) {
    if (_gestureState.pending) {
      final startPosition = _gestureState.start;
      if (startPosition != null && (localPosition - startPosition).distance >= 4) {
        _startDoubleTapDrag(startPosition);
        _updateDoubleTapDragSelection(localPosition);
      }
      return true;
    }

    if (_gestureState.dragging) {
      _updateDoubleTapDragSelection(localPosition);
      return true;
    }

    if (_gestureState.longPressing) {
      _updateLongPress(localPosition);
      return true;
    }

    return false;
  }

  bool _handleResumedPanPointerMove(PointerMoveEvent event) {
    if (_resumedPanState.pointer != event.pointer) {
      return false;
    }

    final previous = _resumedPanState.lastLocalPosition;
    _resumedPanState.lastLocalPosition = event.localPosition;
    if (previous == null) {
      return true;
    }

    if (_isSelecting || _gestureState.active || gesture.hasAnyHandleDrag) {
      _clearResumedPanState();
      return true;
    }

    final delta = event.localPosition - previous;
    if (!_resumedPanState.active) {
      if (delta.distance < 1) {
        return true;
      }
      gesture.startScrollDrag(
        details: DragStartDetails(globalPosition: event.position, localPosition: previous),
        allowHorizontal: _allowHorizontalPan,
      );
      _resumedPanState.active = true;
    }

    if (delta.distance > 0) {
      gesture.updateScrollDrag(
        DragUpdateDetails(
          globalPosition: event.position,
          localPosition: event.localPosition,
          delta: delta,
          sourceTimeStamp: event.timeStamp,
        ),
      );
    }

    return true;
  }

  void onPointerMove(PointerMoveEvent event) {
    _handleInteractionInput(PointerMoveInput(pointer: event.pointer));

    final previousPointerPosition = pinch.pointerPosition(event.pointer);
    if (pinch.containsPointer(event.pointer)) {
      pinch.updatePointer(event.pointer, event.localPosition);
    }

    if (pinch.isPinching) {
      _updatePinchZoom();
      return;
    }

    if (_consumeIfDndLocked(
      onLocked: () {
        if (gesture.hasScrollDrag) {
          gesture.cancelScrollDrag();
        }
        _clearResumedPanState();
      },
    )) {
      return;
    }

    if (_handleResumedPanPointerMove(event)) {
      return;
    }

    if (_handleSelectionPointerMove(event.localPosition)) {
      return;
    }

    final canRawPan =
        pinch.pointerCount == 1 &&
        previousPointerPosition != null &&
        !_isSelecting &&
        !scope.interactionState.snapshot().isAuxiliaryGesture &&
        !gesture.hasAnyHandleDrag &&
        !gesture.hasScrollDrag;
    if (canRawPan) {
      final delta = event.localPosition - previousPointerPosition;
      if (delta.distance >= 1) {
        gesture.applyRawPanDelta(delta: delta, allowHorizontal: _allowHorizontalPan);
      }
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
      _handleInteractionInput(PointerCancelInput(pointer: event.pointer));
    } else {
      _handleInteractionInput(PointerUpInput(pointer: event.pointer));
    }

    if (_resumedPanState.pointer == event.pointer) {
      if (_resumedPanState.active) {
        if (canceled) {
          gesture.cancelScrollDrag();
        } else {
          gesture.endScrollDrag(DragEndDetails());
        }
      }
      _clearResumedPanState();
    }

    final wasPinching = pinch.isPinching;
    pinch.removePointer(event.pointer);
    if (pinch.pointerCount < 2) {
      _endPinchIfNeeded();
    }

    if (wasPinching && pinch.pointerCount == 1) {
      final remaining = pinch.singlePointerEntry;
      if (remaining != null) {
        _resumedPanState.pointer = remaining.key;
        _resumedPanState.lastLocalPosition = remaining.value;
        _resumedPanState.active = false;
      }
    }

    if (wasPinching) {
      return;
    }

    if (_gestureState.dragging) {
      _endDoubleTapDrag();
      return;
    }

    final hadPendingDoubleTap = _gestureState.pending;
    _gestureState.clearPending();
    if (!canceled && hadPendingDoubleTap) {
      final hasRangeSelection = !(scope.controller.state.selection?.collapsed ?? true);
      if (hasRangeSelection) {
        showContextMenu.value = true;
      }
    }
    endLongPress();
    _endTextHandleDrag();
  }
}
