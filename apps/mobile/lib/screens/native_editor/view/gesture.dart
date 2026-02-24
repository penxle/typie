import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/scroll.dart';

class GestureStateMachine {
  GestureStateMachine({required this.isLongPressing});

  final ValueNotifier<bool> isLongPressing;

  Offset? start;
  _GesturePhase _phase = _GesturePhase.idle;

  bool get longPressing => _phase == _GesturePhase.longPress;
  bool get pending => _phase == _GesturePhase.doubleTapPending;
  bool get dragging => _phase == _GesturePhase.doubleTapDragging;
  bool get active => pending || dragging;

  bool startLongPress() {
    if (active) {
      return false;
    }
    _setPhase(_GesturePhase.longPress);
    return true;
  }

  void endLongPress() {
    if (!longPressing) {
      return;
    }
    _setPhase(_GesturePhase.idle);
  }

  void prepare(Offset startPosition) {
    start = startPosition;
    _setPhase(_GesturePhase.doubleTapPending);
  }

  void begin(Offset startPosition) {
    start = startPosition;
    _setPhase(_GesturePhase.doubleTapDragging);
  }

  void clearPending() {
    if (!pending) {
      return;
    }
    _setPhase(_GesturePhase.idle);
  }

  void stop() {
    if (!active) {
      return;
    }
    _setPhase(_GesturePhase.idle);
  }

  void _setPhase(_GesturePhase next) {
    _phase = next;
    final nextLongPressing = _phase == _GesturePhase.longPress;
    if (isLongPressing.value != nextLongPressing) {
      isLongPressing.value = nextLongPressing;
    }
    if (!pending && !dragging) {
      start = null;
    }
  }
}

enum _GesturePhase { idle, longPress, doubleTapPending, doubleTapDragging }

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
    required this.editor,
    required this.controller,
    required this.getPageAtPosition,
    required this.getPointerX,
    required this.getViewportWidth,
    required ValueNotifier<bool> isLongPressing,
  }) : state = GestureStateMachine(isLongPressing: isLongPressing);

  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final NativeEditor editor;
  final EditorController controller;
  final (int, double) Function(double y) getPageAtPosition;
  final double Function(double localX) getPointerX;
  final double Function() getViewportWidth;
  final GestureStateMachine state;

  static const _edgeThreshold = 60.0;
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

  bool get isCellHandleDragging => _draggingCellHandle;
  bool get hasTextHandleDrag => _draggingHandleType != null;
  bool get hasAnyHandleDrag => _draggingCellHandle || hasTextHandleDrag;

  bool get tapDispatched => _tapDispatched;
  SelectionHandleType? get draggingHandleType => _draggingHandleType;
  SelectionHandleInfo? get dragAnchorHandle => _dragAnchorHandle;
  Map<String, dynamic>? get doubleTapInitialRange => _doubleTapInitialRange;

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

  void holdScrollPositions() {
    if (verticalScrollController.hasSingleClient) {
      verticalScrollController.position.hold(() {});
    }
    if (horizontalScrollController.hasSingleClient) {
      horizontalScrollController.position.hold(() {});
    }
  }

  void startScrollDrag({required DragStartDetails details, required bool allowHorizontal}) {
    if (verticalScrollController.hasSingleClient) {
      _verticalDrag = verticalScrollController.position.drag(details, () {
        _verticalDrag = null;
      });
    }
    if (allowHorizontal && horizontalScrollController.hasSingleClient) {
      _horizontalDrag = horizontalScrollController.position.drag(details, () {
        _horizontalDrag = null;
      });
    }
  }

  void updateScrollDrag(DragUpdateDetails details) {
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
  }

  void cancelScrollDrag() {
    _verticalDrag?.cancel();
    _horizontalDrag?.cancel();
    _verticalDrag = null;
    _horizontalDrag = null;
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

      if (_verticalDirection != 0 && verticalScrollController.hasSingleClient) {
        final proximity = 1.0 - (_verticalEdgeDistance / _edgeThreshold).clamp(0.0, 1.0);
        final scrollSpeed = _minScrollSpeed + proximity * (_maxScrollSpeed - _minScrollSpeed);

        final currentOffset = verticalScrollController.offset;
        final newOffset = (currentOffset + _verticalDirection * scrollSpeed).clamp(
          0.0,
          verticalScrollController.position.maxScrollExtent,
        );

        if (newOffset != currentOffset) {
          verticalScrollController.jumpTo(newOffset);
          final viewHeight = _autoScrollViewSize.height;
          scrolledY = _verticalDirection > 0
              ? viewHeight -
                    _edgeThreshold +
                    (newOffset >= verticalScrollController.position.maxScrollExtent ? _edgeThreshold : 0)
              : newOffset.clamp(0.0, _edgeThreshold);
        }
      }

      if (_horizontalDirection != 0 && horizontalScrollController.hasSingleClient) {
        final proximity = 1.0 - (_horizontalEdgeDistance / _edgeThreshold).clamp(0.0, 1.0);
        final scrollSpeed = _minScrollSpeed + proximity * (_maxScrollSpeed - _minScrollSpeed);

        final currentOffset = horizontalScrollController.offset;
        final newOffset = (currentOffset + _horizontalDirection * scrollSpeed).clamp(
          0.0,
          horizontalScrollController.position.maxScrollExtent,
        );

        if (newOffset != currentOffset) {
          horizontalScrollController.jumpTo(newOffset);
          final viewWidth = _autoScrollViewSize.width;
          scrolledX = _horizontalDirection > 0
              ? viewWidth -
                    _edgeThreshold +
                    (newOffset >= horizontalScrollController.position.maxScrollExtent ? _edgeThreshold : 0)
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
          editor.dispatch({'type': 'dragOver', 'pageIdx': pageIdx, 'x': pointerX, 'y': localY});
          return;
        }

        final anchorHandle = _dragAnchorHandle;
        if (_draggingHandleType != null && anchorHandle != null) {
          editor.dispatch({
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
          editor
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
            });
          controller.scrollIntoView();
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
    final scrollOffset = verticalScrollController.hasSingleClient ? verticalScrollController.offset : 0.0;
    final hScrollOffset = horizontalScrollController.hasSingleClient ? horizontalScrollController.offset : 0.0;
    final pageTopOffset = geo.titleAreaHeight + offsets[handle.pageIdx];
    final y = pageTopOffset + handle.y - scrollOffset;
    final x = geo.contentStartX(viewportWidth: getViewportWidth(), horizontalScrollOffset: hScrollOffset) + handle.x;
    return Offset(x, y);
  }

  Offset? getHandleStemCenter(SelectionHandleInfo? handle, ContentGeometry geo) {
    final pos = getHandlePosition(handle, geo);
    if (pos == null || handle == null) {
      return null;
    }
    return Offset(pos.dx, pos.dy + handle.height / 2);
  }

  void dispose() {
    stopAutoScroll();
    state.stop();
    cancelTapTimer();
    cancelScrollDrag();
  }
}

class ConditionalLongPressGestureRecognizer extends LongPressGestureRecognizer {
  ConditionalLongPressGestureRecognizer({required this.condition, super.duration, super.postAcceptSlopTolerance});

  final bool Function(Offset globalPosition) condition;

  @override
  void didExceedDeadline() {
    if (initialPosition == null) {
      super.didExceedDeadline();
      return;
    }

    final globalPosition = initialPosition!.global;
    if (condition(globalPosition)) {
      resolve(GestureDisposition.rejected);
      stopTrackingPointer(primaryPointer!);
    } else {
      super.didExceedDeadline();
    }
  }
}
