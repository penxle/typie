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

class SelectionHandleSemantic implements InteractionSemantic {
  SelectionHandleType? draggingHandleType;
  SelectionHandleType? pendingHandleType;
  Offset? pointerDownTouchPosition;
  Offset? dragStartTouchPosition;
  Offset? dragStartHandleScreenPosition;
  SelectionHandleInfo? dragAnchorHandle;

  bool get hasSelectionHandleDrag => draggingHandleType != null;
  bool get hasAnyHandleDrag => hasSelectionHandleDrag;
  bool get hasPendingSelectionHandleDrag => pendingHandleType != null;

  void clearSelectionHandleState() {
    draggingHandleType = null;
    pendingHandleType = null;
    pointerDownTouchPosition = null;
    dragAnchorHandle = null;
    dragStartTouchPosition = null;
    dragStartHandleScreenPosition = null;
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
    clearSelectionHandleState();
  }
}
