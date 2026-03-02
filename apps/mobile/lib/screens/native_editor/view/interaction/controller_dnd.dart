part of 'controller.dart';

extension ControllerDndMethods on EditorInteractionController {
  void startLocalDndSession(ResolvedDragLocation location) {
    _handleInteractionInput(const DndStartInput(local: true));
    scope.dndController.handleDragStart(
      location.pageIdx,
      location.pointerX,
      location.localY,
      Offset(location.localPosition.dx, location.localPosition.dy),
    );
  }

  void endDndSession() {
    _handleInteractionInput(const DndSessionEndInput());
    dropPosition.value = null;
    gesture.stopAutoScroll();
    scope.dndController.handleDragEnd();
  }

  void beginTableCellHandleDragDown() {
    gesture.startCellHandleDrag();
    scope.longPressPosition.value = null;
  }

  void startTableCellHandleDrag({
    required SelectionHandleInfo? anchorHandle,
    required Offset? viewportPosition,
    required ValueNotifier<Offset?> cellHandleDragPosition,
  }) {
    gesture
      ..stopAutoScroll()
      ..startCellHandleDrag()
      ..setTextHandleDragType(SelectionHandleType.to)
      ..setDragAnchorHandle(anchorHandle);
    _handleInteractionInput(const TableHandleDragStartInput());
    if (viewportPosition != null) {
      cellHandleDragPosition.value = viewportPosition;
    }
  }

  void updateTableCellHandleDrag({
    required Offset viewportPosition,
    required ValueNotifier<Offset?> cellHandleDragPosition,
    required ValueNotifier<Offset?> tableDropPosition,
    required double viewWidth,
    required double viewHeight,
  }) {
    cellHandleDragPosition.value = viewportPosition;
    gesture.handleAutoScroll(
      y: viewportPosition.dy,
      x: viewportPosition.dx,
      viewWidth: viewWidth,
      viewHeight: viewHeight,
      handleDragPosition: cellHandleDragPosition,
      longPressPosition: scope.longPressPosition,
      dropPosition: tableDropPosition,
    );
  }

  void endTableCellHandleDrag({required ValueNotifier<Offset?> cellHandleDragPosition}) {
    gesture
      ..stopCellHandleDrag()
      ..clearSelectionHandleState()
      ..stopAutoScroll();
    _handleInteractionInput(const TableHandleDragEndInput());
    cellHandleDragPosition.value = null;
  }

  DropOperation onDropOver(DropOverEvent event) {
    if (pinch.isPinching) {
      return DropOperation.none;
    }
    _handleInteractionInput(const DndOverInput());

    final item = event.session.items.firstOrNull;
    if (item == null) {
      return DropOperation.none;
    }

    final position = event.position.local;
    final (pageIdx, localY) = getPageAtPosition(position.dy);
    final pointerX = gesture.getPointerX(position.dx);

    dropPosition.value = position;
    gesture.handleAutoScroll(
      y: position.dy,
      x: position.dx,
      viewWidth: readViewWidth(),
      viewHeight: readViewHeight(),
      handleDragPosition: handleDragPosition,
      longPressPosition: longPressPosition,
      dropPosition: dropPosition,
    );
    scope.dndController.handleDragOver(pageIdx, pointerX, localY);

    final localData = item.localData;
    if (localData is Map && localData['isInternal'] == true) {
      return DropOperation.move;
    }
    return DropOperation.copy;
  }

  void onDropEnter(dynamic event) {
    if (pinch.isPinching) {
      return;
    }
    _handleInteractionInput(const DndEnterInput());
    scope.dndController.handleDragEnter();
  }

  void onDropLeave(dynamic event) {
    _handleInteractionInput(const DndLeaveInput());
    dropPosition.value = null;
    gesture.stopAutoScroll();
    scope.dndController.handleDragLeave();
  }

  Future<void> onPerformDrop(PerformDropEvent event) async {
    if (pinch.isPinching) {
      return;
    }

    dropPosition.value = null;
    gesture.stopAutoScroll();

    final position = event.position.local;
    final (pageIdx, localY) = getPageAtPosition(position.dy);
    if (pageIdx < 0) {
      endDndSession();
      return;
    }

    _handleInteractionInput(const DndDropInput());

    final pointerX = gesture.getPointerX(position.dx);
    unawaited(HapticFeedback.lightImpact());
    await scope.dndController.handleDrop(pageIdx: pageIdx, x: pointerX, y: localY, session: event.session);
  }
}
