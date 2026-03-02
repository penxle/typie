part of '../controller.dart';

class SelectionHandleDragContext {
  const SelectionHandleDragContext({
    required this.startTouchPosition,
    required this.startHandleScreenPosition,
    required this.anchorHandle,
  });

  final Offset startTouchPosition;
  final Offset startHandleScreenPosition;
  final SelectionHandleInfo anchorHandle;
}

class HandleDragSession implements InteractionSession {
  SelectionHandleType? draggingHandleType;
  SelectionHandleType? pendingHandleType;
  bool draggingCellHandle = false;
  Offset? pointerDownTouchPosition;
  Offset? dragStartTouchPosition;
  Offset? dragStartHandleScreenPosition;
  SelectionHandleInfo? dragAnchorHandle;
  Map<String, dynamic>? doubleTapInitialRange;

  bool get isCellHandleDragging => draggingCellHandle;
  bool get hasSelectionHandleDrag => draggingHandleType != null;
  bool get hasAnyHandleDrag => draggingCellHandle || hasSelectionHandleDrag;
  bool get hasPendingSelectionHandleDrag => pendingHandleType != null;

  void startCellHandleDrag() {
    draggingCellHandle = true;
  }

  bool stopCellHandleDrag() {
    final wasDragging = draggingCellHandle;
    draggingCellHandle = false;
    return wasDragging;
  }

  void clearSelectionHandleState() {
    draggingHandleType = null;
    pendingHandleType = null;
    dragAnchorHandle = null;
    dragStartTouchPosition = null;
    dragStartHandleScreenPosition = null;
    doubleTapInitialRange = null;
  }

  void setSelectionHandleDragType(SelectionHandleType? type) {
    draggingHandleType = type;
  }

  void setDragAnchorHandle(SelectionHandleInfo? anchorHandle) {
    dragAnchorHandle = anchorHandle;
  }

  void setDoubleTapInitialRange(Map<String, dynamic>? range) {
    doubleTapInitialRange = range;
  }

  void rememberPointerDown(Offset touchPosition) {
    pointerDownTouchPosition = touchPosition;
  }

  void beginPendingSelectionHandleDrag({required SelectionHandleType type, required Offset touchPosition}) {
    pendingHandleType = type;
    pointerDownTouchPosition = touchPosition;
  }

  void clearPendingSelectionHandleDrag() {
    pendingHandleType = null;
  }

  void beginSelectionHandleDrag({
    required SelectionHandleType type,
    required Offset touchPosition,
    required Offset handleScreenPosition,
    required SelectionHandleInfo? anchorHandle,
  }) {
    pendingHandleType = null;
    draggingHandleType = type;
    dragStartTouchPosition = touchPosition;
    dragStartHandleScreenPosition = handleScreenPosition;
    dragAnchorHandle = anchorHandle;
  }

  void beginLongPressSession({
    required Offset touchPosition,
    required Offset? handleScreenPosition,
    required SelectionHandleInfo? anchorHandle,
  }) {
    dragStartTouchPosition = touchPosition;
    dragStartHandleScreenPosition = handleScreenPosition;
    dragAnchorHandle = anchorHandle;
  }

  SelectionHandleDragContext? selectionHandleDragContext() {
    final startTouchPosition = dragStartTouchPosition;
    final startHandleScreenPosition = dragStartHandleScreenPosition;
    final anchorHandle = dragAnchorHandle;
    if (startTouchPosition == null || startHandleScreenPosition == null || anchorHandle == null) {
      return null;
    }

    return SelectionHandleDragContext(
      startTouchPosition: startTouchPosition,
      startHandleScreenPosition: startHandleScreenPosition,
      anchorHandle: anchorHandle,
    );
  }

  Offset? getHandlePosition(
    SelectionHandleInfo? handle,
    ContentGeometry geometry, {
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics Function() resolveHorizontalMetrics,
  }) {
    if (handle == null) {
      return null;
    }

    final offsets = geometry.computeCumulativePageOffsets();
    final scrollOffset = resolveScrollPosition(verticalScrollController)?.pixels ?? 0.0;
    final horizontalMetrics = resolveHorizontalMetrics();
    final hScrollOffset = horizontalMetrics.scrollOffset;
    final pageTopOffset = geometry.titleAreaHeight + offsets[handle.pageIdx];
    final y = pageTopOffset + geometry.toDisplayY(handle.y) - scrollOffset;
    final x =
        geometry.contentStartX(
          viewportWidth: horizontalMetrics.viewportDimension,
          horizontalScrollOffset: hScrollOffset,
        ) +
        geometry.toDisplayX(handle.x);
    return Offset(x, y);
  }

  Offset? getHandleStemCenter(
    SelectionHandleInfo? handle,
    ContentGeometry geometry, {
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics Function() resolveHorizontalMetrics,
  }) {
    final position = getHandlePosition(
      handle,
      geometry,
      verticalScrollController: verticalScrollController,
      resolveHorizontalMetrics: resolveHorizontalMetrics,
    );
    if (position == null || handle == null) {
      return null;
    }

    return Offset(position.dx, position.dy + geometry.toDisplayY(handle.height) / 2);
  }

  @override
  void reset() {
    draggingHandleType = null;
    pendingHandleType = null;
    draggingCellHandle = false;
    pointerDownTouchPosition = null;
    dragStartTouchPosition = null;
    dragStartHandleScreenPosition = null;
    dragAnchorHandle = null;
    doubleTapInitialRange = null;
  }
}

extension HandleDragInteractionMethods on EditorInteractionController {
  void _endSelectionHandleDrag() {
    _handleDragSession.clearPendingSelectionHandleDrag();
    final hadHandleDrag = _handleDragSession.hasSelectionHandleDrag || handleDragPosition.value != null;
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

    _handleDragSession.beginPendingSelectionHandleDrag(
      type: type,
      touchPosition: renderBox.globalToLocal(details.globalPosition),
    );
  }

  void onHandleDragStart(SelectionHandleType type, DragStartDetails details) {
    if (!_decide(command: InteractionCommand.selectionHandleStart)) {
      _handleDragSession.clearPendingSelectionHandleDrag();
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
        _handleDragSession.pointerDownTouchPosition ?? renderBox.globalToLocal(details.globalPosition);
    final handle = type == SelectionHandleType.from ? fromHandle : toHandle;

    _handleDragSession.beginSelectionHandleDrag(
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

    _handleAutoScroll(y: touchPosition.dy, x: touchPosition.dx);
  }

  void onHandleDragEnd(SelectionHandleType type, DragEndDetails details) {
    _handleDragSession.clearPendingSelectionHandleDrag();
    _endSelectionHandleDrag();
  }
}
