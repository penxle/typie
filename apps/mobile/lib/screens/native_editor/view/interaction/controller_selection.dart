part of 'controller.dart';

extension ControllerSelectionMethods on EditorInteractionController {
  bool _isConsecutiveTap({required Offset localPosition, required DateTime now}) {
    return gesture.isConsecutiveTap(localPosition: localPosition, now: now);
  }

  void _endTextHandleDrag() {
    if (pinch.isPinching || gesture.isCellHandleDragging) {
      return;
    }

    final hadHandleDrag = gesture.hasTextHandleDrag || handleDragPosition.value != null;
    if (!hadHandleDrag) {
      return;
    }

    gesture.stopSelectionHandlesAndAutoScroll();
    handleDragPosition.value = null;
    _handleInteractionInput(const TextHandleDragEndInput());
  }

  bool _dispatchDoubleTapSelection(Offset localPosition) {
    if (pinch.isPinching) {
      return false;
    }

    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    if (pageIdx < 0) {
      return false;
    }

    showContextMenu.value = false;
    scope.inputController
      ..commitComposing()
      ..openInput();

    final pointerX = gesture.getPointerX(localPosition.dx);
    scope.controller.dispatch({
      'type': 'pointerDown',
      'pageIdx': pageIdx,
      'x': pointerX,
      'y': localY,
      'clickCount': 2,
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

    gesture.clearTapHistory();
    scope.controller.scrollIntoView();
    return true;
  }

  void _prepareDoubleTapDrag(Offset localPosition) {
    if (pinch.isPinching) {
      return;
    }

    gesture
      ..cancelTapTimer()
      ..setTapDispatched(true)
      ..clearSelectionHandleState()
      ..stopAutoScroll();

    longPressPosition.value = null;
    showContextMenu.value = false;
    _gestureState.prepare(localPosition);
    handleDragPosition.value = null;
  }

  void _startDoubleTapDrag(Offset localPosition) {
    if (pinch.isPinching) {
      return;
    }

    gesture
      ..cancelTapTimer()
      ..setTapDispatched(true)
      ..setTextHandleDragType(SelectionHandleType.to)
      ..stopAutoScroll();

    longPressPosition.value = null;
    showContextMenu.value = false;
    _gestureState.begin(localPosition);
    handleDragPosition.value = null;
    _handleInteractionInput(const DoubleTapDragStartInput());
  }

  void _endDoubleTapDrag() {
    _gestureState.stop();
    _endTextHandleDrag();
    _handleInteractionInput(const DoubleTapDragEndInput());
  }

  ({SelectionHandleInfo anchor, Map<String, dynamic> initialRange})? _resolveDoubleTapDragSelectionContext() {
    final dragAnchorHandle = gesture.dragAnchorHandle;
    final doubleTapInitialRange = gesture.doubleTapInitialRange;
    if (dragAnchorHandle != null && doubleTapInitialRange != null) {
      return (anchor: dragAnchorHandle, initialRange: doubleTapInitialRange);
    }

    final selection = scope.controller.state.selection;
    if (selection == null || selection.collapsed) {
      return null;
    }

    final anchor = selection.fromBounds;
    if (anchor == null) {
      return null;
    }

    final initialRange = selection.range;
    gesture
      ..setDragAnchorHandle(anchor)
      ..setDoubleTapInitialRange(initialRange);
    return (anchor: anchor, initialRange: initialRange);
  }

  void _updateDoubleTapDragSelection(Offset localPosition) {
    if (pinch.isPinching || !_gestureState.dragging) {
      return;
    }

    final startPosition = _gestureState.start;
    if (startPosition != null && (localPosition - startPosition).distance < 4) {
      return;
    }

    handleDragPosition.value = localPosition;
    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    final pointerX = gesture.getPointerX(localPosition.dx);
    final selectionContext = _resolveDoubleTapDragSelectionContext();

    if (selectionContext == null || pageIdx < 0) {
      return;
    }

    scope.controller.dispatch({
      'type': 'extendSelectionTo',
      'anchorPageIdx': selectionContext.anchor.pageIdx,
      'anchorX': selectionContext.anchor.x,
      'anchorY': selectionContext.anchor.y + selectionContext.anchor.height / 2,
      'headPageIdx': pageIdx,
      'headX': pointerX,
      'headY': localY,
      'doubleTapInitialRange': selectionContext.initialRange,
    });
    gesture.handleAutoScroll(
      y: localPosition.dy,
      x: localPosition.dx,
      viewWidth: readViewWidth(),
      viewHeight: readViewHeight(),
      handleDragPosition: handleDragPosition,
      longPressPosition: longPressPosition,
      dropPosition: dropPosition,
    );
  }

  void onHandleDragDown(SelectionHandleType type, DragDownDetails details) {
    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return;
    }

    gesture.rememberPointerDown(renderBox.globalToLocal(details.globalPosition));
  }

  void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
    if (pinch.isPinching) {
      return;
    }

    _handleInteractionInput(const TextHandleDragStartInput());

    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return;
    }

    final state = scope.controller.state;
    final fromHandle = state.selection?.fromBounds;
    final toHandle = state.selection?.toBounds;

    final touchPosition = gesture.pointerDownTouchPosition() ?? renderBox.globalToLocal(details.globalPosition);
    final handle = type == SelectionHandleType.from ? fromHandle : toHandle;

    gesture.beginTextHandleDrag(
      type: type,
      touchPosition: touchPosition,
      handleScreenPosition: gesture.getHandleStemCenter(handle, readGeometry()) ?? touchPosition,
      anchorHandle: type == SelectionHandleType.from ? toHandle : fromHandle,
    );
  }

  void onHandleDragUpdate(SelectionHandleType type, DragUpdateDetails details) {
    if (pinch.isPinching) {
      return;
    }

    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return;
    }

    final touchPosition = renderBox.globalToLocal(details.globalPosition);
    final dragContext = gesture.selectionHandleDragContext();
    if (dragContext == null) {
      return;
    }

    final delta = touchPosition - dragContext.startTouchPosition;
    final selectionScreenPosition = dragContext.startHandleScreenPosition + delta;

    handleDragPosition.value = selectionScreenPosition;

    final (pageIdx, localY) = getPageAtPosition(selectionScreenPosition.dy);
    if (pageIdx >= 0) {
      final pointerX = gesture.getPointerX(selectionScreenPosition.dx);
      scope.controller.dispatch({
        'type': 'extendSelectionTo',
        'anchorPageIdx': dragContext.anchorHandle.pageIdx,
        'anchorX': dragContext.anchorHandle.x,
        'anchorY': dragContext.anchorHandle.y + dragContext.anchorHandle.height / 2,
        'headPageIdx': pageIdx,
        'headX': pointerX,
        'headY': localY,
      });
    }

    gesture.handleAutoScroll(
      y: touchPosition.dy,
      x: touchPosition.dx,
      viewWidth: readViewWidth(),
      viewHeight: readViewHeight(),
      handleDragPosition: handleDragPosition,
      longPressPosition: longPressPosition,
      dropPosition: dropPosition,
    );
  }

  void onHandleDragEnd(SelectionHandleType type, DragEndDetails details) {
    _endTextHandleDrag();
  }

  void startLongPress(Offset globalPosition) {
    if (pinch.isPinching || gesture.isCellHandleDragging || _gestureState.active) {
      return;
    }

    final viewportPosition = viewportPositionFromGlobal(globalPosition);
    if (viewportPosition == null) {
      return;
    }

    scope.inputController.commitComposing();

    longPressPosition.value = viewportPosition;
    if (!_gestureState.startLongPress()) {
      return;
    }

    _handleInteractionInput(const LongPressStartInput());

    final state = scope.controller.state;
    final draggingHandle = state.draggingHandle;
    final anchorHandle = draggingHandle == SelectionHandleType.from
        ? state.selection?.toBounds
        : state.selection?.fromBounds;

    gesture.beginLongPressSession(
      touchPosition: globalPosition,
      handleScreenPosition: gesture.getHandleStemCenter(
        state.selection?.fromBounds ?? state.selection?.toBounds,
        readGeometry(),
      ),
      anchorHandle: anchorHandle,
    );
  }

  void _updateLongPress(Offset viewportPosition) {
    if (pinch.isPinching || !_gestureState.longPressing || _gestureState.active) {
      return;
    }

    final (pageIdx, localY) = getPageAtPosition(viewportPosition.dy);
    longPressPosition.value = viewportPosition;

    if (pageIdx >= 0) {
      final pointerX = gesture.getPointerX(viewportPosition.dx);
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

    gesture.handleAutoScroll(
      y: viewportPosition.dy,
      x: viewportPosition.dx,
      viewWidth: readViewWidth(),
      viewHeight: readViewHeight(),
      handleDragPosition: handleDragPosition,
      longPressPosition: longPressPosition,
      dropPosition: dropPosition,
    );
  }

  void endLongPress() {
    if (pinch.isPinching || !_gestureState.longPressing || _gestureState.active) {
      return;
    }

    longPressPosition.value = null;
    gesture.stopAutoScroll();
    _gestureState.endLongPress();
    _handleInteractionInput(const LongPressEndInput());
  }

  void _dispatchTap(Offset localPosition) {
    if (pinch.isPinching) {
      return;
    }

    showContextMenu.value = false;

    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    if (pageIdx < 0) {
      return;
    }

    scope.inputController
      ..commitComposing()
      ..openInput();

    final now = DateTime.now();
    final clickCount = _isConsecutiveTap(localPosition: localPosition, now: now) ? 2 : 1;

    gesture.recordTap(now: now, localPosition: localPosition);

    final pointerX = gesture.getPointerX(localPosition.dx);
    final tappedInteractive = scope.editor.isInteractiveHit(pageIdx, pointerX, localY);

    if (clickCount == 1) {
      final isSelectionHit = scope.editor.isSelectionHit(pageIdx, pointerX, localY);
      if (isSelectionHit) {
        if (!wasContextMenuOpen.value) {
          showContextMenu.value = true;
        }
        return;
      }
    }

    final keysPressed = HardwareKeyboard.instance.logicalKeysPressed;
    final isShiftHeader =
        keysPressed.contains(LogicalKeyboardKey.shiftLeft) || keysPressed.contains(LogicalKeyboardKey.shiftRight);

    final prevCursor = scope.controller.state.cursor;

    scope.controller.dispatch({
      'type': 'pointerDown',
      'pageIdx': pageIdx,
      'x': pointerX,
      'y': localY,
      'clickCount': clickCount,
      'button': 'primary',
      'modifier': {'shift': isShiftHeader, 'ctrl': false, 'alt': false, 'meta': false},
    });
    scope.controller.dispatch({
      'type': 'pointerUp',
      'pageIdx': pageIdx,
      'x': pointerX,
      'y': localY,
      'button': 'primary',
      'modifier': {'shift': isShiftHeader, 'ctrl': false, 'alt': false, 'meta': false},
    });

    if (clickCount != 1) {
      scope.controller.scrollIntoView();
      return;
    }

    unawaited(
      scope.ticker.settled().then((_) {
        if (!context.mounted) {
          return;
        }

        final newState = scope.controller.state;
        final isCollapsed = newState.selection?.collapsed ?? true;

        final isSameCursor =
            isCollapsed && newState.cursor != null && prevCursor != null && newState.cursor!.isSamePosition(prevCursor);

        if (isSameCursor) {
          if (!tappedInteractive && !wasContextMenuOpen.value) {
            showContextMenu.value = true;
          }
          return;
        }

        if (tappedInteractive) {
          return;
        }

        scope.controller.scrollIntoView();
      }),
    );
  }

  void onTapDown(TapDownDetails details) {
    if (pinch.isPinching) {
      return;
    }

    wasContextMenuOpen.value = showContextMenu.value;
    if (showContextMenu.value) {
      showContextMenu.value = false;
    }

    gesture.cancelTapTimer();

    if (_isConsecutiveTap(localPosition: details.localPosition, now: DateTime.now())) {
      gesture.setTapDispatched(true);
      if (_dispatchDoubleTapSelection(details.localPosition)) {
        _prepareDoubleTapDrag(details.localPosition);
      }
      return;
    }

    gesture
      ..setTapDispatched(false)
      ..scheduleTapTimer(const Duration(milliseconds: 150), () {
        final pointerX = gesture.getPointerX(details.localPosition.dx);
        final (pageIdx, localY) = gesture.getPageAtPosition(details.localPosition.dy);

        final canDrag = gesture.controller.editor.isSelectionHit(pageIdx, pointerX, localY);
        if (canDrag) {
          gesture.setTapDispatched(true);
          return;
        }

        final hasRangeSelection = !(scope.controller.state.selection?.collapsed ?? true);
        if (hasRangeSelection) {
          return;
        }

        gesture.setTapDispatched(true);
        _dispatchTap(details.localPosition);
      });
  }

  void onTapUp(TapUpDetails details) {
    if (pinch.isPinching || _gestureState.dragging) {
      return;
    }

    _gestureState.clearPending();
    gesture.cancelTapTimer();
    if (!gesture.tapDispatched) {
      _dispatchTap(details.localPosition);
    }
  }

  void onTapCancel() {
    if (pinch.isPinching || _gestureState.active) {
      return;
    }

    _gestureState.clearPending();
    gesture.cancelTapTimer();
  }
}
