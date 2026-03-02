part of '../controller.dart';

class DndSession implements InteractionSession {
  bool _active = false;
  bool _local = false;
  bool _nativeLocalDragActive = false;

  bool get isActive => _active;
  bool get isLocal => _local;
  bool get isNativeLocalDragActive => _nativeLocalDragActive;

  void startLocal() {
    _active = true;
    _local = true;
    _nativeLocalDragActive = true;
  }

  void startExternalIfIdle() {
    if (_active) {
      return;
    }
    _active = true;
    _local = false;
    _nativeLocalDragActive = false;
  }

  void endNativeLocalDrag() {
    _nativeLocalDragActive = false;
  }

  @override
  void reset() {
    _active = false;
    _local = false;
    _nativeLocalDragActive = false;
  }
}

extension DndInteractionMethods on EditorInteractionController {
  void _endNativeLocalDragIfNeeded() {
    if (!_dndSession.isNativeLocalDragActive) {
      return;
    }
    scope.dndController.handleDragEnd();
    _dndSession.endNativeLocalDrag();
  }

  void onLocalDragCompleted(DropOperation operation) {
    if (!_dndSession.isActive || !_dndSession.isLocal) {
      return;
    }

    if (operation == DropOperation.none ||
        operation == DropOperation.userCancelled ||
        operation == DropOperation.forbidden) {
      endDndSession();
    }
  }

  void startLocalDndSession(ResolvedDragLocation location) {
    _dndSession.startLocal();
    if (!_decide(
      command: InteractionCommand.dndBeginLocal,
      transitionEvent: const InteractionEvent.dndStart(local: true),
      expectedMode: InteractionMode.dndLocal,
    )) {
      _dndSession.reset();
      return;
    }

    scope.dndController.handleDragStart(
      location.pageIdx,
      location.pointerX,
      location.localY,
      Offset(location.localPosition.dx, location.localPosition.dy),
    );
  }

  void endDndSession() {
    _applyTransition(InteractionEvent.dndSessionEnd);
    dropPosition.value = null;
    _autoScrollSession.stop();
    _endNativeLocalDragIfNeeded();
    _dndSession.reset();
  }

  bool beginTableCellHandleDragDown() {
    if (!_decide(command: InteractionCommand.tableCellHandleBeginDown)) {
      return false;
    }
    _handleDragSession.startCellHandleDrag();
    scope.longPressPosition.value = null;
    return true;
  }

  bool startTableCellHandleDrag({
    required SelectionHandleInfo? anchorHandle,
    required Offset? viewportPosition,
    required ValueNotifier<Offset?> cellHandleDragPosition,
  }) {
    _autoScrollSession.stop();
    _handleDragSession
      ..startCellHandleDrag()
      ..setSelectionHandleDragType(SelectionHandleType.to)
      ..setDragAnchorHandle(anchorHandle);

    if (!_decide(
      command: InteractionCommand.tableCellHandleBeginDragging,
      transitionEvent: InteractionEvent.tableHandleDragStart,
      expectedMode: InteractionMode.tableCellHandleDragging,
    )) {
      _handleDragSession
        ..stopCellHandleDrag()
        ..clearSelectionHandleState();
      return false;
    }

    if (viewportPosition != null) {
      cellHandleDragPosition.value = viewportPosition;
    }
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
    _autoScrollSession.handle(
      y: viewportPosition.dy,
      x: viewportPosition.dx,
      viewWidth: viewWidth,
      viewHeight: viewHeight,
      handleDragPosition: cellHandleDragPosition,
      longPressPosition: scope.longPressPosition,
      dropPosition: tableDropPosition,
      verticalScrollController: scope.verticalScrollController,
      resolveHorizontalMetrics: _resolveHorizontalMetrics,
      getPageAtPosition: getPageAtPosition,
      getPointerX: _resolvePointerX,
      readDraggingHandleType: () => _handleDragSession.draggingHandleType,
      readDragAnchorHandle: () => _handleDragSession.dragAnchorHandle,
      readDoubleTapInitialRange: () => _handleDragSession.doubleTapInitialRange,
      dispatch: scope.controller.dispatch,
      scrollIntoView: scope.controller.scrollIntoView,
    );
    return true;
  }

  bool endTableCellHandleDrag({required ValueNotifier<Offset?> cellHandleDragPosition}) {
    _handleDragSession
      ..stopCellHandleDrag()
      ..clearSelectionHandleState();
    _autoScrollSession.stop();
    final ended = _decide(
      command: InteractionCommand.tableCellHandleEnd,
      transitionEvent: InteractionEvent.tableHandleDragEnd,
      expectedMode: InteractionMode.idle,
    );
    cellHandleDragPosition.value = null;
    return ended;
  }

  DropOperation onDropOver(DropOverEvent event) {
    if (!_decide(command: InteractionCommand.dndHandleDropOver)) {
      return DropOperation.none;
    }
    _applyTransition(InteractionEvent.dndOver);

    final item = event.session.items.firstOrNull;
    if (!_decide(command: InteractionCommand.dndHandleDropOverItem(hasItem: item != null))) {
      return DropOperation.none;
    }
    final dropItem = item;
    if (dropItem == null) {
      return DropOperation.none;
    }

    final position = event.position.local;
    final (pageIdx, localY) = getPageAtPosition(position.dy);
    final pointerX = _resolvePointerX(position.dx);

    dropPosition.value = position;
    _handleAutoScroll(y: position.dy, x: position.dx);
    scope.dndController.handleDragOver(pageIdx, pointerX, localY);

    final localData = dropItem.localData;
    if (localData is Map && localData['isInternal'] == true) {
      return DropOperation.move;
    }
    return DropOperation.copy;
  }

  void onDropEnter(dynamic event) {
    if (!_decide(command: InteractionCommand.dndHandleDropEnter)) {
      return;
    }

    _dndSession.startExternalIfIdle();
    if (!_decide(command: InteractionCommand.dndBeginExternal, transitionEvent: InteractionEvent.dndEnter)) {
      return;
    }

    scope.dndController.handleDragEnter();
  }

  void onDropLeave(dynamic event) {
    _applyTransition(InteractionEvent.dndLeave);
    dropPosition.value = null;
    _autoScrollSession.stop();
    scope.dndController.handleDragLeave();
    if (!_dndSession.isNativeLocalDragActive) {
      _dndSession.reset();
    }
  }

  void onDropEnded(dynamic event) {
    if (_decide(command: InteractionCommand.dndShouldEndOnDropEnded)) {
      endDndSession();
      return;
    }
    _endNativeLocalDragIfNeeded();
    _dndSession.reset();
  }

  Future<void> onPerformDrop(PerformDropEvent event) async {
    if (!_decide(command: InteractionCommand.dndPerformDrop)) {
      return;
    }

    dropPosition.value = null;
    _autoScrollSession.stop();

    final position = event.position.local;
    final (pageIdx, localY) = getPageAtPosition(position.dy);
    if (!_decide(command: InteractionCommand.dndPerformDropOnPage(pageIdx: pageIdx))) {
      endDndSession();
      return;
    }

    _applyTransition(InteractionEvent.dndDrop);

    final pointerX = _resolvePointerX(position.dx);
    unawaited(HapticFeedback.lightImpact());
    final result = await scope.dndController.handleDrop(
      pageIdx: pageIdx,
      x: pointerX,
      y: localY,
      session: event.session,
    );
    if (result == DndDropResult.needsDragEnd) {
      scope.dndController.handleDragEnd();
      _dndSession.endNativeLocalDrag();
    }
    _endNativeLocalDragIfNeeded();
    _dndSession.reset();
  }
}
