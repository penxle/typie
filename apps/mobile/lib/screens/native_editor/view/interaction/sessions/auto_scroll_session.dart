part of '../controller.dart';

class AutoScrollSession implements InteractionSession {
  Timer? _autoScrollTimer;
  double _verticalEdgeDistance = 0;
  double _horizontalEdgeDistance = 0;
  double _verticalDirection = 0;
  double _horizontalDirection = 0;
  Size _autoScrollViewSize = Size.zero;
  (int, double, double)? _lastDispatchedPosition;

  static const _edgeThreshold = 30.0;
  static const _minScrollSpeed = 4.0;
  static const _maxScrollSpeed = 16.0;

  bool get isActive => _autoScrollTimer != null;

  void stop() {
    _autoScrollTimer?.cancel();
    _autoScrollTimer = null;
    _verticalDirection = 0;
    _horizontalDirection = 0;
    _lastDispatchedPosition = null;
  }

  void handle({
    required double y,
    required double x,
    required double viewWidth,
    required double viewHeight,
    required ValueNotifier<Offset?> handleDragPosition,
    required ValueNotifier<Offset?> longPressPosition,
    required ValueNotifier<Offset?> dropPosition,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics Function() resolveHorizontalMetrics,
    required (int pageIdx, double localY) Function(double y) getPageAtPosition,
    required double Function(double localX) getPointerX,
    required SelectionHandleType? Function() readDraggingHandleType,
    required SelectionHandleInfo? Function() readDragAnchorHandle,
    required Map<String, dynamic>? Function() readDoubleTapInitialRange,
    required void Function(Map<String, dynamic> event) dispatch,
    required VoidCallback scrollIntoView,
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
      _start(
        handleDragPosition: handleDragPosition,
        longPressPosition: longPressPosition,
        dropPosition: dropPosition,
        verticalScrollController: verticalScrollController,
        resolveHorizontalMetrics: resolveHorizontalMetrics,
        getPageAtPosition: getPageAtPosition,
        getPointerX: getPointerX,
        readDraggingHandleType: readDraggingHandleType,
        readDragAnchorHandle: readDragAnchorHandle,
        readDoubleTapInitialRange: readDoubleTapInitialRange,
        dispatch: dispatch,
        scrollIntoView: scrollIntoView,
      );
      return;
    }

    stop();
  }

  void _start({
    required ValueNotifier<Offset?> handleDragPosition,
    required ValueNotifier<Offset?> longPressPosition,
    required ValueNotifier<Offset?> dropPosition,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics Function() resolveHorizontalMetrics,
    required (int pageIdx, double localY) Function(double y) getPageAtPosition,
    required double Function(double localX) getPointerX,
    required SelectionHandleType? Function() readDraggingHandleType,
    required SelectionHandleInfo? Function() readDragAnchorHandle,
    required Map<String, dynamic>? Function() readDoubleTapInitialRange,
    required void Function(Map<String, dynamic> event) dispatch,
    required VoidCallback scrollIntoView,
  }) {
    if (_autoScrollTimer != null) {
      return;
    }

    _autoScrollTimer = Timer.periodic(const Duration(milliseconds: 16), (_) {
      final activePosition = dropPosition.value ?? handleDragPosition.value ?? longPressPosition.value;
      var scrolledY = activePosition?.dy ?? 0;
      var scrolledX = activePosition?.dx ?? 0;

      final verticalPosition = resolveScrollPosition(verticalScrollController);
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

      final horizontalMetrics = resolveHorizontalMetrics();
      final horizontalPosition = horizontalMetrics.activePosition;
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
        stop();
        return;
      }

      if (activePosition == null) {
        return;
      }

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
        dispatch({'type': 'dragOver', 'pageIdx': pageIdx, 'x': pointerX, 'y': localY});
        return;
      }

      final draggingHandleType = readDraggingHandleType();
      final dragAnchorHandle = readDragAnchorHandle();
      if (draggingHandleType != null && dragAnchorHandle != null) {
        dispatch({
          'type': 'extendSelectionTo',
          'anchorPageIdx': dragAnchorHandle.pageIdx,
          'anchorX': dragAnchorHandle.x,
          'anchorY': dragAnchorHandle.y + dragAnchorHandle.height / 2,
          'headPageIdx': pageIdx,
          'headX': pointerX,
          'headY': localY,
          if (readDoubleTapInitialRange() != null) 'doubleTapInitialRange': readDoubleTapInitialRange(),
        });
        return;
      }

      if (draggingHandleType == null) {
        dispatch({
          'type': 'pointerDown',
          'pageIdx': pageIdx,
          'x': pointerX,
          'y': localY,
          'clickCount': 1,
          'button': 'primary',
          'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
        });
        dispatch({
          'type': 'pointerUp',
          'pageIdx': pageIdx,
          'x': pointerX,
          'y': localY,
          'button': 'primary',
          'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
        });
        scrollIntoView();
      }
    });
  }

  @override
  void reset() {
    stop();
    _verticalEdgeDistance = 0;
    _horizontalEdgeDistance = 0;
    _autoScrollViewSize = Size.zero;
  }
}
