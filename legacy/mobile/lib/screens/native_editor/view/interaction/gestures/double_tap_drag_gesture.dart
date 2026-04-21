part of '../controller.dart';

class DoubleTapDragGesture implements InteractionGesture {
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

extension DoubleTapDragGestureMethods on EditorInteractionController {
  bool _dispatchDoubleTapSelection(Offset localPosition) {
    final (pageIdx, _) = getPageAtPosition(localPosition.dy);
    if (!_decide(command: InteractionCommand.doubleTapDispatchSelection(pageIdx: pageIdx))) {
      return false;
    }

    if (!_semanticSelectWordAt(localPosition)) {
      return false;
    }

    _tapGesture.clearTapHistory();
    return true;
  }

  bool prepareDoubleTapDrag(Offset localPosition) {
    if (!_decide(command: InteractionCommand.doubleTapPrepareDrag)) {
      return false;
    }

    _tapGesture
      ..cancelTapTimer()
      ..tapDispatched = true;
    _clearSelectionExpansionState();
    _clearLongPressState();
    _dismissSelectionUi();
    _doubleTapDragGesture.prepare(localPosition);
    return true;
  }

  bool startDoubleTapDrag(Offset localPosition) {
    if (!_decide(command: InteractionCommand.doubleTapStartDrag)) {
      return false;
    }

    _tapGesture
      ..cancelTapTimer()
      ..tapDispatched = true;

    _doubleTapDragGesture.begin(localPosition);
    handleDragPosition.value = null;
    if (!_decide(
      command: InteractionCommand.doubleTapBeginSelecting,
      transitionEvent: InteractionEvent.doubleTapDragStart,
      expectedMode: InteractionMode.doubleTapSelecting,
    )) {
      _doubleTapDragGesture.stop();
      return false;
    }
    return true;
  }

  bool endDoubleTapDrag() {
    final wasActive =
        _doubleTapDragGesture.active || scope.interactionState.snapshot().mode == InteractionMode.doubleTapSelecting;
    _doubleTapDragGesture.stop();
    _clearSelectionExpansionState();
    _autoScrollSemantic.stop();
    _applyTransition(InteractionEvent.doubleTapDragEnd);
    return wasActive;
  }

  bool updateDoubleTapDragSelection(Offset localPosition) {
    if (!_decide(
      command: InteractionCommand.doubleTapUpdateSelection(
        localPosition: localPosition,
        dragStartPosition: _doubleTapDragGesture.start,
      ),
    )) {
      return false;
    }

    handleDragPosition.value = localPosition;
    final (pageIdx, localY) = getPageAtPosition(localPosition.dy);
    final pointerX = _resolvePointerX(localPosition.dx);
    final selectionContext = _resolveWordSelectionDragContext();

    if (!_decide(
      command: InteractionCommand.doubleTapExtendSelection(
        pageIdx: pageIdx,
        hasSelectionContext: selectionContext != null,
      ),
    )) {
      return false;
    }

    final resolvedSelectionContext = selectionContext!;
    _semanticExtendSelectionTo(
      anchor: resolvedSelectionContext.anchor,
      headPageIdx: pageIdx,
      headX: pointerX,
      headY: localY,
      initialRange: resolvedSelectionContext.initialRange,
    );

    _handleAutoScroll(y: localPosition.dy, x: localPosition.dx);
    return true;
  }
}
