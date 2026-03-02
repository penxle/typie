part of '../controller.dart';

class DoubleTapDragSession implements InteractionSession {
  Offset? start;
  _DoubleTapDragPhase _phase = _DoubleTapDragPhase.idle;

  bool get pending => _phase == _DoubleTapDragPhase.pending;
  bool get dragging => _phase == _DoubleTapDragPhase.dragging;
  bool get active => pending || dragging;

  void prepare(Offset startPosition) {
    start = startPosition;
    _phase = _DoubleTapDragPhase.pending;
  }

  void begin(Offset startPosition) {
    start = startPosition;
    _phase = _DoubleTapDragPhase.dragging;
  }

  void clearPending() {
    if (!pending) {
      return;
    }
    _phase = _DoubleTapDragPhase.idle;
    start = null;
  }

  void stop() {
    if (!active) {
      return;
    }
    _phase = _DoubleTapDragPhase.idle;
    start = null;
  }

  @override
  void reset() {
    _phase = _DoubleTapDragPhase.idle;
    start = null;
  }
}

enum _DoubleTapDragPhase { idle, pending, dragging }

extension DoubleTapDragInteractionMethods on EditorInteractionController {
  bool _dispatchDoubleTapSelection(Offset localPosition) {
    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    if (!_decide(command: InteractionCommand.doubleTapDispatchSelection(pageIdx: pageIdx))) {
      return false;
    }

    showContextMenu.value = false;
    scope.inputController
      ..commitComposing()
      ..openInput();

    final pointerX = _resolvePointerX(localPosition.dx);
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

    _tapSession.clearTapHistory();
    scope.controller.scrollIntoView();
    return true;
  }

  bool prepareDoubleTapDrag(Offset localPosition) {
    if (!_decide(command: InteractionCommand.doubleTapPrepareDrag)) {
      return false;
    }

    _tapSession
      ..cancelTapTimer()
      ..tapDispatched = true;
    _handleDragSession.clearSelectionHandleState();
    _autoScrollSession.stop();

    longPressPosition.value = null;
    showContextMenu.value = false;
    _doubleTapDragSession.prepare(localPosition);
    handleDragPosition.value = null;
    return true;
  }

  bool startDoubleTapDrag(Offset localPosition) {
    if (!_decide(command: InteractionCommand.doubleTapStartDrag)) {
      return false;
    }

    _tapSession
      ..cancelTapTimer()
      ..tapDispatched = true;
    _handleDragSession
      ..setSelectionHandleDragType(SelectionHandleType.to)
      ..dragStartTouchPosition = localPosition;
    _autoScrollSession.stop();

    longPressPosition.value = null;
    showContextMenu.value = false;
    _doubleTapDragSession.begin(localPosition);
    handleDragPosition.value = null;
    if (!_decide(
      command: InteractionCommand.doubleTapBeginSelecting,
      transitionEvent: InteractionEvent.doubleTapDragStart,
      expectedMode: InteractionMode.doubleTapSelecting,
    )) {
      _doubleTapDragSession.stop();
      return false;
    }
    return true;
  }

  bool endDoubleTapDrag() {
    final wasActive =
        _doubleTapDragSession.active || scope.interactionState.snapshot().mode == InteractionMode.doubleTapSelecting;
    _doubleTapDragSession.stop();
    _endSelectionHandleDrag();
    _applyTransition(InteractionEvent.doubleTapDragEnd);
    return wasActive;
  }

  ({SelectionHandleInfo anchor, Map<String, dynamic> initialRange})? _resolveDoubleTapDragSelectionContext() {
    final dragAnchorHandle = _handleDragSession.dragAnchorHandle;
    final doubleTapInitialRange = _handleDragSession.doubleTapInitialRange;
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
    _handleDragSession
      ..setDragAnchorHandle(anchor)
      ..setDoubleTapInitialRange(initialRange);
    return (anchor: anchor, initialRange: initialRange);
  }

  bool updateDoubleTapDragSelection(Offset localPosition) {
    if (!_decide(
      command: InteractionCommand.doubleTapUpdateSelection(
        localPosition: localPosition,
        dragStartPosition: _doubleTapDragSession.start,
      ),
    )) {
      return false;
    }

    handleDragPosition.value = localPosition;
    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    final pointerX = _resolvePointerX(localPosition.dx);
    final selectionContext = _resolveDoubleTapDragSelectionContext();

    if (!_decide(
      command: InteractionCommand.doubleTapExtendSelection(
        pageIdx: pageIdx,
        hasSelectionContext: selectionContext != null,
      ),
    )) {
      return false;
    }

    final resolvedSelectionContext = selectionContext!;
    scope.controller.dispatch({
      'type': 'extendSelectionTo',
      'anchorPageIdx': resolvedSelectionContext.anchor.pageIdx,
      'anchorX': resolvedSelectionContext.anchor.x,
      'anchorY': resolvedSelectionContext.anchor.y + resolvedSelectionContext.anchor.height / 2,
      'headPageIdx': pageIdx,
      'headX': pointerX,
      'headY': localY,
      'doubleTapInitialRange': resolvedSelectionContext.initialRange,
    });

    _handleAutoScroll(y: localPosition.dy, x: localPosition.dx);
    return true;
  }
}
