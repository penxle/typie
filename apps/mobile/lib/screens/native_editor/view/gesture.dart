import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';

class GestureController {
  GestureController({
    required this.verticalScrollController,
    required this.horizontalScrollController,
    required this.editor,
    required this.controller,
    required this.getPageAtPosition,
    required this.getPointerX,
    required this.getHorizontalPadding,
  });

  final ScrollController verticalScrollController;
  final ScrollController horizontalScrollController;
  final NativeEditor editor;
  final EditorController controller;
  final (int, double) Function(double y) getPageAtPosition;
  final double Function(double localX) getPointerX;
  final double Function() getHorizontalPadding;

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

  SelectionHandleType? draggingHandleType;
  bool draggingCellHandle = false;
  Offset? pointerDownTouchPosition;
  Offset? dragStartTouchPosition;
  Offset? dragStartHandleScreenPosition;
  SelectionHandleInfo? dragAnchorHandle;

  DateTime? lastTapTime;
  Offset? lastTapPosition;
  Timer? tapTimer;
  bool tapDispatched = false;

  Drag? verticalDrag;
  Drag? horizontalDrag;

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

      if (_verticalDirection != 0 && verticalScrollController.hasClients) {
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

      if (_horizontalDirection != 0 && horizontalScrollController.hasClients) {
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

        final hOffset = horizontalScrollController.hasClients ? horizontalScrollController.offset : 0.0;
        final pointerX = scrolledX + hOffset - getHorizontalPadding();

        final currentPosition = (pageIdx, pointerX, localY);
        if (_lastDispatchedPosition == currentPosition) {
          return;
        }
        _lastDispatchedPosition = currentPosition;

        if (dropPosition.value != null) {
          editor.dispatch({'type': 'dragOver', 'pageIdx': pageIdx, 'x': pointerX, 'y': localY});
          return;
        }

        final anchorHandle = dragAnchorHandle;
        if (draggingHandleType != null && anchorHandle != null) {
          editor.dispatch({
            'type': 'extendSelectionTo',
            'anchorPageIdx': anchorHandle.pageIdx,
            'anchorX': anchorHandle.x,
            'anchorY': anchorHandle.y + anchorHandle.height / 2,
            'headPageIdx': pageIdx,
            'headX': pointerX,
            'headY': localY,
          });
        } else if (draggingHandleType == null) {
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
    final scrollOffset = verticalScrollController.hasClients ? verticalScrollController.offset : 0.0;
    final hScrollOffset = horizontalScrollController.hasClients ? horizontalScrollController.offset : 0.0;
    final pageTopOffset = geo.titleAreaHeight + offsets[handle.pageIdx];
    final y = pageTopOffset + handle.y - scrollOffset;
    final x = geo.horizontalPadding + handle.x - hScrollOffset;
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
    tapTimer?.cancel();
    tapTimer = null;
    verticalDrag?.cancel();
    horizontalDrag?.cancel();
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
