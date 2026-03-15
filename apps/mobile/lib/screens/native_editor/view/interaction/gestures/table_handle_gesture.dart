part of '../controller.dart';

extension TableHandleGestureMethods on EditorInteractionController {
  bool beginTableCellHandleDragDown(DragDownDetails details) {
    if (!_decide(command: InteractionCommand.tableCellHandleBeginDown)) {
      return false;
    }

    final renderBox = _interactionRenderBox();
    if (renderBox == null) {
      return false;
    }

    _selectionHandleSemantic.beginPendingSelectionHandleDrag(
      type: SelectionHandleType.to,
      touchPosition: renderBox.globalToLocal(details.globalPosition),
    );
    scope.longPressPosition.value = null;
    return true;
  }

  bool startTableCellHandleDrag({
    required SelectionHandleInfo? anchorHandle,
    required Offset? viewportPosition,
    required ValueNotifier<Offset?> cellHandleDragPosition,
  }) {
    _autoScrollSemantic.stop();
    if (!_selectionHandleSemantic.hasPendingSelectionHandleDrag) {
      return false;
    }

    if (!_decide(
      command: InteractionCommand.tableCellHandleBeginDragging,
      transitionEvent: InteractionEvent.tableHandleDragStart,
      expectedMode: InteractionMode.tableCellHandleDragging,
    )) {
      _selectionHandleSemantic.clearPendingSelectionHandleDrag();
      return false;
    }

    final pendingTouchPosition = _selectionHandleSemantic.pointerDownTouchPosition;
    final touchPosition = pendingTouchPosition ?? viewportPosition;
    if (touchPosition == null) {
      _selectionHandleSemantic.clearSelectionHandleState();
      return false;
    }

    final handleScreenPosition = viewportPosition ?? touchPosition;
    _selectionHandleSemantic.beginSelectionHandleDrag(
      type: SelectionHandleType.to,
      touchPosition: touchPosition,
      handleScreenPosition: handleScreenPosition,
      anchorHandle: anchorHandle,
    );
    cellHandleDragPosition.value = handleScreenPosition;
    return true;
  }

  bool updateTableCellHandleDrag({
    required Offset viewportPosition,
    required ValueNotifier<Offset?> cellHandleDragPosition,
    required ValueNotifier<Offset?> tableDropPosition,
    required double viewWidth,
    required double viewHeight,
  }) {
    if (!_decide(command: InteractionCommand.tableCellHandleUpdate)) {
      return false;
    }

    cellHandleDragPosition.value = viewportPosition;
    _autoScrollSemantic.handle(
      y: viewportPosition.dy,
      x: viewportPosition.dx,
      visibleArea: scope.visibleEditorArea,
      handleDragPosition: cellHandleDragPosition,
      longPressPosition: scope.longPressPosition,
      dropPosition: tableDropPosition,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
      getPageAtPosition: getPageAtPosition,
      getPointerX: _resolvePointerX,
      readSelectionContext: () => (anchor: null, initialRange: null, blockCursorFallback: false),
      dispatch: scope.controller.dispatch,
      scrollIntoView: scope.controller.scrollIntoView,
    );
    return true;
  }

  bool endTableCellHandleDrag({ValueNotifier<Offset?>? cellHandleDragPosition}) {
    _selectionHandleSemantic
      ..clearPendingSelectionHandleDrag()
      ..clearSelectionHandleState();
    _autoScrollSemantic.stop();
    final ended = _decide(
      command: InteractionCommand.tableCellHandleEnd,
      transitionEvent: InteractionEvent.tableHandleDragEnd,
      expectedMode: InteractionMode.idle,
    );
    cellHandleDragPosition?.value = null;
    return ended;
  }
}
