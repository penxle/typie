part of '../controller.dart';

extension SelectionHandleGestureMethods on EditorInteractionController {
  void _endSelectionHandleDrag() {
    _selectionHandleSemantic.clearPendingSelectionHandleDrag();
    final hadHandleDrag =
        scope.interactionState.snapshot().mode == InteractionMode.selectionHandleDragging ||
        _selectionHandleSemantic.hasSelectionHandleDrag ||
        handleDragPosition.value != null;
    if (!_decide(command: InteractionCommand.selectionHandleEnd(hasActiveDrag: hadHandleDrag))) {
      return;
    }

    stopSelectionHandlesAndAutoScroll();
    handleDragPosition.value = null;
    _applyTransition(InteractionEvent.selectionHandleDragEnd);
  }

  void onHandleDragDown(SelectionHandleType type, DragDownDetails details) {
    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return;
    }

    _selectionHandleSemantic.beginPendingSelectionHandleDrag(
      type: type,
      touchPosition: renderBox.globalToLocal(details.globalPosition),
    );
  }

  void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
    if (!_decide(command: InteractionCommand.selectionHandleStart)) {
      _selectionHandleSemantic.clearPendingSelectionHandleDrag();
      return;
    }

    if (!_decide(
      command: InteractionCommand.selectionHandleBeginDragging,
      transitionEvent: InteractionEvent.selectionHandleDragStart,
      expectedMode: InteractionMode.selectionHandleDragging,
    )) {
      return;
    }

    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return;
    }

    final state = scope.controller.state;
    final fromHandle = state.selection?.fromBounds;
    final toHandle = state.selection?.toBounds;

    final touchPosition =
        _selectionHandleSemantic.pointerDownTouchPosition ?? renderBox.globalToLocal(details.globalPosition);
    final handle = type == SelectionHandleType.from ? fromHandle : toHandle;

    _selectionHandleSemantic.beginSelectionHandleDrag(
      type: type,
      touchPosition: touchPosition,
      handleScreenPosition: _handleStemCenter(handle, readGeometry()) ?? touchPosition,
      anchorHandle: type == SelectionHandleType.from ? toHandle : fromHandle,
    );
  }

  void onHandleDragUpdate(SelectionHandleType type, DragUpdateDetails details) {
    if (!_decide(command: InteractionCommand.selectionHandleUpdate)) {
      return;
    }

    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return;
    }

    final touchPosition = renderBox.globalToLocal(details.globalPosition);
    final dragContext = _selectionHandleDragContext();
    if (dragContext == null) {
      return;
    }

    final delta = touchPosition - dragContext.startTouchPosition;
    final selectionScreenPosition = dragContext.startHandleScreenPosition + delta;

    handleDragPosition.value = selectionScreenPosition;

    final (pageIdx, localY) = getPageAtPosition(selectionScreenPosition.dy);
    if (pageIdx >= 0) {
      final pointerX = _resolvePointerX(selectionScreenPosition.dx);
      _semanticExtendSelectionTo(
        anchor: dragContext.anchorHandle,
        headPageIdx: pageIdx,
        headX: pointerX,
        headY: localY,
      );
    }

    _handleAutoScroll(y: touchPosition.dy, x: touchPosition.dx);
  }

  void onHandleDragEnd(SelectionHandleType type, DragEndDetails details) {
    _selectionHandleSemantic.clearPendingSelectionHandleDrag();
    _endSelectionHandleDrag();
  }
}
