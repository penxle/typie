part of '../controller.dart';

class AutoScrollSemantic implements InteractionSemantic {
  Timer? _autoScrollTimer;
  double _verticalEdgeDistance = 0;
  double _horizontalEdgeDistance = 0;
  double _verticalDirection = 0;
  double _horizontalDirection = 0;
  (int, double, double)? _lastDispatchedPosition;

  static const _edgeThreshold = 30.0;
  static const _minScrollSpeed = 4.0;
  static const _maxScrollSpeed = 16.0;

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
    required VisibleEditorArea visibleArea,
    required ValueNotifier<Offset?> handleDragPosition,
    required ValueNotifier<Offset?> longPressPosition,
    required ValueNotifier<Offset?> dropPosition,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics Function() resolveHorizontalMetrics,
    required (int pageIdx, double localY) Function(double y) getPageAtPosition,
    required double Function(double localX) getPointerX,
    required AutoScrollSelectionContext Function() readSelectionContext,
    required void Function(Map<String, dynamic> event) dispatch,
    required VoidCallback scrollIntoView,
  }) {
    final visiblePosition = visibleArea.localToVisible(Offset(x, y));
    const topThreshold = _edgeThreshold;
    final bottomThreshold = visibleArea.visibleHeight - _edgeThreshold;

    if (visiblePosition.dy < topThreshold) {
      _verticalEdgeDistance = visiblePosition.dy.clamp(0.0, _edgeThreshold);
      _verticalDirection = -1;
    } else if (visiblePosition.dy > bottomThreshold) {
      _verticalEdgeDistance = (visibleArea.visibleHeight - visiblePosition.dy).clamp(0.0, _edgeThreshold);
      _verticalDirection = 1;
    } else {
      _verticalDirection = 0;
    }

    if (visiblePosition.dx < _edgeThreshold) {
      _horizontalEdgeDistance = visiblePosition.dx;
      _horizontalDirection = -1;
    } else if (visiblePosition.dx > visibleArea.width - _edgeThreshold) {
      _horizontalEdgeDistance = visibleArea.width - visiblePosition.dx;
      _horizontalDirection = 1;
    } else {
      _horizontalDirection = 0;
    }

    if (_verticalDirection != 0 || _horizontalDirection != 0) {
      _start(
        visibleArea: visibleArea,
        handleDragPosition: handleDragPosition,
        longPressPosition: longPressPosition,
        dropPosition: dropPosition,
        verticalScrollController: verticalScrollController,
        resolveHorizontalMetrics: resolveHorizontalMetrics,
        getPageAtPosition: getPageAtPosition,
        getPointerX: getPointerX,
        readSelectionContext: readSelectionContext,
        dispatch: dispatch,
        scrollIntoView: scrollIntoView,
      );
      return;
    }

    stop();
  }

  void _start({
    required VisibleEditorArea visibleArea,
    required ValueNotifier<Offset?> handleDragPosition,
    required ValueNotifier<Offset?> longPressPosition,
    required ValueNotifier<Offset?> dropPosition,
    required ScrollController verticalScrollController,
    required HorizontalScrollMetrics Function() resolveHorizontalMetrics,
    required (int pageIdx, double localY) Function(double y) getPageAtPosition,
    required double Function(double localX) getPointerX,
    required AutoScrollSelectionContext Function() readSelectionContext,
    required void Function(Map<String, dynamic> event) dispatch,
    required VoidCallback scrollIntoView,
  }) {
    if (_autoScrollTimer != null) {
      return;
    }

    _autoScrollTimer = Timer.periodic(const Duration(milliseconds: 16), (_) {
      final activePosition = dropPosition.value ?? handleDragPosition.value ?? longPressPosition.value;
      var scrolledVisibleY = activePosition == null ? 0.0 : visibleArea.localToVisibleY(activePosition.dy);
      var scrolledVisibleX = activePosition == null ? 0.0 : visibleArea.localToVisible(activePosition).dx;

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
          scrolledVisibleY = _verticalDirection > 0
              ? (visibleArea.visibleHeight -
                        _edgeThreshold +
                        (newOffset >= verticalPosition.maxScrollExtent ? _edgeThreshold : 0))
                    .clamp(0.0, visibleArea.visibleHeight)
              : (newOffset <= 0 ? 0.0 : _edgeThreshold).clamp(0.0, visibleArea.visibleHeight);
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
          scrolledVisibleX = _horizontalDirection > 0
              ? visibleArea.width -
                    _edgeThreshold +
                    (newOffset >= horizontalPosition.maxScrollExtent ? _edgeThreshold : 0)
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

      final scrolledY = visibleArea.visibleToLocalY(scrolledVisibleY);
      final scrolledX = visibleArea.visibleToLocal(Offset(scrolledVisibleX, 0)).dx;
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

      final selectionContext = readSelectionContext();
      final selectionAnchor = selectionContext.anchor;
      if (selectionAnchor != null) {
        dispatch(
          buildExtendSelectionEvent(
            anchor: selectionAnchor,
            headPageIdx: pageIdx,
            headX: pointerX,
            headY: localY,
            initialRange: selectionContext.initialRange,
          ),
        );
        return;
      }

      if (selectionContext.blockCursorFallback) {
        return;
      }

      dispatch(
        buildPrimaryPointerDownEvent(
          pageIdx: pageIdx,
          pointerX: pointerX,
          localY: localY,
          clickCount: 1,
          isShiftPressed: false,
        ),
      );
      dispatch(buildPrimaryPointerUpEvent(pageIdx: pageIdx, pointerX: pointerX, localY: localY, isShiftPressed: false));
      scrollIntoView();
    });
  }

  @override
  void reset() {
    stop();
    _verticalEdgeDistance = 0;
    _horizontalEdgeDistance = 0;
  }
}
