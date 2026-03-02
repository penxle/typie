import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

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

class GestureController {
  GestureController({
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.controller,
    required this.getPageAtPosition,
    required this.getPointerX,
    required this.getHorizontalMetrics,
  });

  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final EditorController controller;
  final (int, double) Function(double y) getPageAtPosition;
  final double Function(double localX) getPointerX;
  final HorizontalScrollMetrics Function() getHorizontalMetrics;

  static const _edgeThreshold = 30.0;
  static const _minScrollSpeed = 4.0;
  static const _maxScrollSpeed = 16.0;

  Timer? _autoScrollTimer;
  double _verticalEdgeDistance = 0;
  double _horizontalEdgeDistance = 0;
  double _verticalDirection = 0;
  double _horizontalDirection = 0;
  Size _autoScrollViewSize = Size.zero;
  (int, double, double)? _lastDispatchedPosition;

  SelectionHandleType? _draggingHandleType;
  bool _draggingCellHandle = false;
  Offset? _pointerDownTouchPosition;
  Offset? _dragStartTouchPosition;
  Offset? _dragStartHandleScreenPosition;
  SelectionHandleInfo? _dragAnchorHandle;
  Map<String, dynamic>? _doubleTapInitialRange;

  DateTime? _lastTapTime;
  Offset? _lastTapPosition;
  Timer? _tapTimer;
  bool _tapDispatched = false;

  Drag? _verticalDrag;
  Drag? _horizontalDrag;
  bool _horizontalPanEnabled = false;

  bool get isCellHandleDragging => _draggingCellHandle;
  bool get hasTextHandleDrag => _draggingHandleType != null;
  bool get hasAnyHandleDrag => _draggingCellHandle || hasTextHandleDrag;
  bool get hasScrollDrag => _verticalDrag != null || _horizontalDrag != null;

  bool get tapDispatched => _tapDispatched;
  SelectionHandleType? get draggingHandleType => _draggingHandleType;
  SelectionHandleInfo? get dragAnchorHandle => _dragAnchorHandle;
  Map<String, dynamic>? get doubleTapInitialRange => _doubleTapInitialRange;

  ScrollPosition? get _verticalPosition {
    return resolveScrollPosition(verticalScrollController);
  }

  HorizontalScrollMetrics get _horizontalMetrics => getHorizontalMetrics();

  ScrollPosition? get _horizontalPosition {
    return _horizontalMetrics.activePosition;
  }

  void startCellHandleDrag() {
    _draggingCellHandle = true;
  }

  bool stopCellHandleDrag() {
    final wasDragging = _draggingCellHandle;
    _draggingCellHandle = false;
    return wasDragging;
  }

  void clearSelectionHandleState() {
    _draggingHandleType = null;
    _dragAnchorHandle = null;
    _dragStartTouchPosition = null;
    _dragStartHandleScreenPosition = null;
    _doubleTapInitialRange = null;
  }

  void stopSelectionHandlesAndAutoScroll() {
    clearSelectionHandleState();
    stopAutoScroll();
  }

  void setTextHandleDragType(SelectionHandleType? type) {
    _draggingHandleType = type;
  }

  void setDragAnchorHandle(SelectionHandleInfo? anchorHandle) {
    _dragAnchorHandle = anchorHandle;
  }

  void setDoubleTapInitialRange(Map<String, dynamic>? range) {
    _doubleTapInitialRange = range;
  }

  void rememberPointerDown(Offset touchPosition) {
    _pointerDownTouchPosition = touchPosition;
  }

  void beginTextHandleDrag({
    required SelectionHandleType type,
    required Offset touchPosition,
    required Offset handleScreenPosition,
    required SelectionHandleInfo? anchorHandle,
  }) {
    _draggingHandleType = type;
    _dragStartTouchPosition = touchPosition;
    _dragStartHandleScreenPosition = handleScreenPosition;
    _dragAnchorHandle = anchorHandle;
  }

  void beginLongPressSession({
    required Offset touchPosition,
    required Offset? handleScreenPosition,
    required SelectionHandleInfo? anchorHandle,
  }) {
    _dragStartTouchPosition = touchPosition;
    _dragStartHandleScreenPosition = handleScreenPosition;
    _dragAnchorHandle = anchorHandle;
    clearTapHistory();
  }

  SelectionHandleDragContext? selectionHandleDragContext() {
    final startTouchPosition = _dragStartTouchPosition;
    final startHandleScreenPosition = _dragStartHandleScreenPosition;
    final anchorHandle = _dragAnchorHandle;
    if (startTouchPosition == null || startHandleScreenPosition == null || anchorHandle == null) {
      return null;
    }
    return SelectionHandleDragContext(
      startTouchPosition: startTouchPosition,
      startHandleScreenPosition: startHandleScreenPosition,
      anchorHandle: anchorHandle,
    );
  }

  Offset? pointerDownTouchPosition() => _pointerDownTouchPosition;

  bool isConsecutiveTap({
    required Offset localPosition,
    required DateTime now,
    int maxTapIntervalMs = 300,
    double maxTapDistance = 20,
  }) {
    final prevTime = _lastTapTime;
    final prevPosition = _lastTapPosition;
    if (prevTime == null || prevPosition == null) {
      return false;
    }

    final timeDiff = now.difference(prevTime).inMilliseconds;
    final distance = (localPosition - prevPosition).distance;
    return timeDiff < maxTapIntervalMs && distance < maxTapDistance;
  }

  void recordTap({required DateTime now, required Offset localPosition}) {
    _lastTapTime = now;
    _lastTapPosition = localPosition;
  }

  void clearTapHistory() {
    _lastTapTime = null;
    _lastTapPosition = null;
  }

  void cancelTapTimer() {
    _tapTimer?.cancel();
    _tapTimer = null;
  }

  void scheduleTapTimer(Duration duration, VoidCallback onTimeout) {
    cancelTapTimer();
    _tapTimer = Timer(duration, onTimeout);
  }

  void setTapDispatched(bool dispatched) {
    _tapDispatched = dispatched;
  }

  void startScrollDrag({required DragStartDetails details, required bool allowHorizontal}) {
    final horizontalMetrics = _horizontalMetrics;
    final horizontalPosition = horizontalMetrics.activePosition;
    final canStartHorizontal = allowHorizontal && horizontalMetrics.canScrollHorizontally && horizontalPosition != null;

    _horizontalPanEnabled = canStartHorizontal;
    final verticalPosition = _verticalPosition;
    if (verticalPosition != null) {
      _verticalDrag = verticalPosition.drag(details, () {
        _verticalDrag = null;
      });
    }
    if (canStartHorizontal) {
      _horizontalDrag = horizontalPosition.drag(details, () {
        _horizontalDrag = null;
      });
    }
  }

  void updateScrollDrag(DragUpdateDetails details) {
    final horizontalMetrics = _horizontalMetrics;
    final horizontalPosition = horizontalMetrics.activePosition;
    final canFallbackHorizontal =
        _horizontalPanEnabled &&
        horizontalMetrics.canScrollHorizontally &&
        horizontalPosition != null &&
        details.delta.dx != 0;
    final horizontalBefore = canFallbackHorizontal ? horizontalPosition.pixels : null;

    _verticalDrag?.update(
      DragUpdateDetails(
        globalPosition: details.globalPosition,
        delta: Offset(0, details.delta.dy),
        primaryDelta: details.delta.dy,
        sourceTimeStamp: details.sourceTimeStamp,
      ),
    );
    _horizontalDrag?.update(
      DragUpdateDetails(
        globalPosition: details.globalPosition,
        delta: Offset(details.delta.dx, 0),
        primaryDelta: details.delta.dx,
        sourceTimeStamp: details.sourceTimeStamp,
      ),
    );

    if (canFallbackHorizontal) {
      final horizontalAfterDrag = horizontalPosition.pixels;
      final dragMoved = horizontalBefore != null && (horizontalAfterDrag - horizontalBefore).abs() > 0.01;
      if (!dragMoved) {
        final nextOffset = (horizontalAfterDrag - details.delta.dx).clamp(0.0, horizontalPosition.maxScrollExtent);
        if ((nextOffset - horizontalAfterDrag).abs() > 0) {
          horizontalPosition.jumpTo(nextOffset);
        }
      }
    }
  }

  void applyRawPanDelta({required Offset delta, required bool allowHorizontal}) {
    final verticalPosition = _verticalPosition;
    if (verticalPosition != null &&
        verticalPosition.hasContentDimensions &&
        verticalPosition.maxScrollExtent > 0 &&
        delta.dy != 0) {
      final currentOffset = verticalPosition.pixels;
      final nextOffset = (currentOffset - delta.dy).clamp(0.0, verticalPosition.maxScrollExtent);
      if ((nextOffset - currentOffset).abs() > 0) {
        verticalPosition.jumpTo(nextOffset);
      }
    }

    final horizontalMetrics = _horizontalMetrics;
    final horizontalPosition = horizontalMetrics.activePosition;
    if (allowHorizontal && horizontalMetrics.canScrollHorizontally && horizontalPosition != null && delta.dx != 0) {
      final currentOffset = horizontalPosition.pixels;
      final nextOffset = (currentOffset - delta.dx).clamp(0.0, horizontalPosition.maxScrollExtent);
      if ((nextOffset - currentOffset).abs() > 0) {
        horizontalPosition.jumpTo(nextOffset);
      }
    }
  }

  void endScrollDrag(DragEndDetails details) {
    _verticalDrag?.end(
      DragEndDetails(
        velocity: Velocity(pixelsPerSecond: Offset(0, details.velocity.pixelsPerSecond.dy)),
        primaryVelocity: details.velocity.pixelsPerSecond.dy,
      ),
    );
    _horizontalDrag?.end(
      DragEndDetails(
        velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
        primaryVelocity: details.velocity.pixelsPerSecond.dx,
      ),
    );
    _verticalDrag = null;
    _horizontalDrag = null;
    _horizontalPanEnabled = false;
  }

  void cancelScrollDrag() {
    _verticalDrag?.cancel();
    _horizontalDrag?.cancel();
    _verticalDrag = null;
    _horizontalDrag = null;
    _horizontalPanEnabled = false;
  }

  void stopAutoScroll() {
    _autoScrollTimer?.cancel();
    _autoScrollTimer = null;
    _verticalDirection = 0;
    _horizontalDirection = 0;
    _lastDispatchedPosition = null;
  }

  void startAutoScroll({
    required ValueNotifier<Offset?> handleDragPosition,
    required ValueNotifier<Offset?> longPressPosition,
    required ValueNotifier<Offset?> dropPosition,
  }) {
    if (_autoScrollTimer != null) {
      return;
    }
    _autoScrollTimer = Timer.periodic(const Duration(milliseconds: 16), (_) {
      final activePosition = dropPosition.value ?? handleDragPosition.value ?? longPressPosition.value;
      var scrolledY = activePosition?.dy ?? 0;
      var scrolledX = activePosition?.dx ?? 0;

      final verticalPosition = _verticalPosition;
      if (_verticalDirection != 0 && verticalPosition != null && verticalPosition.hasContentDimensions) {
        final proximity = 1.0 - (_verticalEdgeDistance / _edgeThreshold).clamp(0.0, 1.0);
        final scrollSpeed = _minScrollSpeed + proximity * (_maxScrollSpeed - _minScrollSpeed);

        final currentOffset = verticalPosition.pixels;
        final newOffset = (currentOffset + _verticalDirection * scrollSpeed).clamp(
          0.0,
          verticalPosition.maxScrollExtent,
        );

        if (newOffset != currentOffset) {
          verticalPosition.jumpTo(newOffset);
          final viewHeight = _autoScrollViewSize.height;
          scrolledY = _verticalDirection > 0
              ? viewHeight - _edgeThreshold + (newOffset >= verticalPosition.maxScrollExtent ? _edgeThreshold : 0)
              : newOffset.clamp(0.0, _edgeThreshold);
        }
      }

      final horizontalPosition = _horizontalPosition;
      if (_horizontalDirection != 0 && horizontalPosition != null && horizontalPosition.hasContentDimensions) {
        final proximity = 1.0 - (_horizontalEdgeDistance / _edgeThreshold).clamp(0.0, 1.0);
        final scrollSpeed = _minScrollSpeed + proximity * (_maxScrollSpeed - _minScrollSpeed);

        final currentOffset = horizontalPosition.pixels;
        final newOffset = (currentOffset + _horizontalDirection * scrollSpeed).clamp(
          0.0,
          horizontalPosition.maxScrollExtent,
        );

        if (newOffset != currentOffset) {
          horizontalPosition.jumpTo(newOffset);
          final viewWidth = _autoScrollViewSize.width;
          scrolledX = _horizontalDirection > 0
              ? viewWidth - _edgeThreshold + (newOffset >= horizontalPosition.maxScrollExtent ? _edgeThreshold : 0)
              : newOffset.clamp(0.0, _edgeThreshold);
        }
      }

      if (_verticalDirection == 0 && _horizontalDirection == 0) {
        stopAutoScroll();
        return;
      }

      if (activePosition != null) {
        final (pageIdx, localY) = getPageAtPosition(scrolledY);

        if (pageIdx < 0) {
          return;
        }

        final pointerX = getPointerX(scrolledX);

        final currentPosition = (pageIdx, pointerX, localY);
        if (_lastDispatchedPosition == currentPosition) {
          return;
        }
        _lastDispatchedPosition = currentPosition;

        if (dropPosition.value != null) {
          controller.dispatch({'type': 'dragOver', 'pageIdx': pageIdx, 'x': pointerX, 'y': localY});
          return;
        }

        final anchorHandle = _dragAnchorHandle;
        if (_draggingHandleType != null && anchorHandle != null) {
          controller.dispatch({
            'type': 'extendSelectionTo',
            'anchorPageIdx': anchorHandle.pageIdx,
            'anchorX': anchorHandle.x,
            'anchorY': anchorHandle.y + anchorHandle.height / 2,
            'headPageIdx': pageIdx,
            'headX': pointerX,
            'headY': localY,
            if (_doubleTapInitialRange != null) 'doubleTapInitialRange': _doubleTapInitialRange,
          });
        } else if (_draggingHandleType == null) {
          controller
            ..dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'clickCount': 1,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            })
            ..dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIdx,
              'x': pointerX,
              'y': localY,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            })
            ..scrollIntoView();
        }
      }
    });
  }

  void handleAutoScroll({
    required double y,
    required double x,
    required double viewWidth,
    required double viewHeight,
    required ValueNotifier<Offset?> handleDragPosition,
    required ValueNotifier<Offset?> longPressPosition,
    required ValueNotifier<Offset?> dropPosition,
  }) {
    _autoScrollViewSize = Size(viewWidth, viewHeight);

    if (y < _edgeThreshold) {
      _verticalEdgeDistance = y;
      _verticalDirection = -1;
    } else if (y > viewHeight - _edgeThreshold) {
      _verticalEdgeDistance = viewHeight - y;
      _verticalDirection = 1;
    } else {
      _verticalDirection = 0;
    }

    if (x < _edgeThreshold) {
      _horizontalEdgeDistance = x;
      _horizontalDirection = -1;
    } else if (x > viewWidth - _edgeThreshold) {
      _horizontalEdgeDistance = viewWidth - x;
      _horizontalDirection = 1;
    } else {
      _horizontalDirection = 0;
    }

    if (_verticalDirection != 0 || _horizontalDirection != 0) {
      startAutoScroll(
        handleDragPosition: handleDragPosition,
        longPressPosition: longPressPosition,
        dropPosition: dropPosition,
      );
    } else {
      stopAutoScroll();
    }
  }

  Offset? getHandlePosition(SelectionHandleInfo? handle, ContentGeometry geo) {
    if (handle == null) {
      return null;
    }
    final offsets = geo.computeCumulativePageOffsets();
    final scrollOffset = _verticalPosition?.pixels ?? 0.0;
    final horizontalMetrics = _horizontalMetrics;
    final hScrollOffset = horizontalMetrics.scrollOffset;
    final pageTopOffset = geo.titleAreaHeight + offsets[handle.pageIdx];
    final y = pageTopOffset + geo.toDisplayY(handle.y) - scrollOffset;
    final x =
        geo.contentStartX(viewportWidth: horizontalMetrics.viewportDimension, horizontalScrollOffset: hScrollOffset) +
        geo.toDisplayX(handle.x);
    return Offset(x, y);
  }

  Offset? getHandleStemCenter(SelectionHandleInfo? handle, ContentGeometry geo) {
    final pos = getHandlePosition(handle, geo);
    if (pos == null || handle == null) {
      return null;
    }
    return Offset(pos.dx, pos.dy + geo.toDisplayY(handle.height) / 2);
  }

  void dispose() {
    stopAutoScroll();
    cancelTapTimer();
    cancelScrollDrag();
  }
}
