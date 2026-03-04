part of '../controller.dart';

typedef PinchPageResolver = (int pageIdx, double localY) Function(double y);

class PinchViewportGesture implements InteractionGesture {
  PinchAnchor? _anchor;
  PinchScrollTarget? _pendingTarget;
  double _pendingContentWidth = 0;
  double _pendingViewportWidth = 0;
  bool _syncScheduled = false;

  @override
  void reset() {
    _anchor = null;
    _pendingTarget = null;
    _pendingContentWidth = 0;
    _pendingViewportWidth = 0;
    _syncScheduled = false;
  }

  void captureAnchor({required int pageIdx, required double logicalX, required double logicalY}) {
    _anchor = (pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);
  }

  void ensureAnchor({required int pageIdx, required double logicalX, required double logicalY}) {
    _anchor ??= (pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);
  }

  void syncViewport({
    required Offset focal,
    required ContentGeometry geometry,
    required double viewportWidth,
    required ScrollController horizontalScrollController,
    required ScrollController verticalScrollController,
    required bool Function() isMounted,
    required bool Function() isPinching,
  }) {
    final anchor = _anchor;
    if (anchor == null) {
      return;
    }

    final horizontalMetrics = resolveHorizontalScrollMetrics(
      controller: horizontalScrollController,
      contentWidth: geometry.contentWidth,
      fallbackViewportDimension: viewportWidth,
    );
    final verticalPosition = resolveScrollPosition(verticalScrollController);

    final effectiveViewportWidth = horizontalMetrics.viewportDimension;

    final baseStartX = geometry.contentStartX(viewportWidth: effectiveViewportWidth, horizontalScrollOffset: 0);
    final targetHorizontal = baseStartX + geometry.toDisplayX(anchor.logicalX) - focal.dx;

    var targetVertical = verticalPosition?.pixels ?? 0.0;
    if (anchor.pageIdx >= 0 && geometry.pages.isNotEmpty) {
      final offsets = geometry.computeCumulativePageOffsets();
      final clampedPageIdx = anchor.pageIdx.clamp(0, geometry.pages.length - 1);
      final pageTop = offsets[clampedPageIdx];
      targetVertical = geometry.titleAreaHeight + pageTop + geometry.toDisplayY(anchor.logicalY) - focal.dy;
    }

    _pendingTarget = (horizontal: targetHorizontal, vertical: targetVertical);
    _pendingContentWidth = geometry.contentWidth;
    _pendingViewportWidth = effectiveViewportWidth;
    _applyPendingTarget(
      horizontalScrollController: horizontalScrollController,
      verticalScrollController: verticalScrollController,
    );
    _scheduleSync(
      horizontalScrollController: horizontalScrollController,
      verticalScrollController: verticalScrollController,
      isMounted: isMounted,
      isPinching: isPinching,
    );
  }

  void _applyPendingTarget({
    required ScrollController horizontalScrollController,
    required ScrollController verticalScrollController,
  }) {
    final target = _pendingTarget;
    if (target == null) {
      return;
    }

    final horizontalPosition = resolveHorizontalScrollMetrics(
      controller: horizontalScrollController,
      contentWidth: _pendingContentWidth,
      fallbackViewportDimension: _pendingViewportWidth,
    ).position;
    if (horizontalPosition != null && horizontalPosition.hasContentDimensions) {
      final clampedHorizontal = target.horizontal.clamp(0.0, horizontalPosition.maxScrollExtent);
      if ((horizontalPosition.pixels - clampedHorizontal).abs() > pinchScrollJumpThreshold) {
        horizontalPosition.jumpTo(clampedHorizontal);
      }
    }

    final verticalPosition = resolveScrollPosition(verticalScrollController);
    if (verticalPosition != null && verticalPosition.hasContentDimensions) {
      final clampedVertical = target.vertical.clamp(0.0, verticalPosition.maxScrollExtent);
      if ((verticalPosition.pixels - clampedVertical).abs() > pinchScrollJumpThreshold) {
        verticalPosition.jumpTo(clampedVertical);
      }
    }
  }

  void _scheduleSync({
    required ScrollController horizontalScrollController,
    required ScrollController verticalScrollController,
    required bool Function() isMounted,
    required bool Function() isPinching,
  }) {
    if (_syncScheduled) {
      return;
    }
    _syncScheduled = true;

    WidgetsBinding.instance.addPostFrameCallback((_) {
      _syncScheduled = false;
      if (!isMounted() || !isPinching()) {
        return;
      }
      _applyPendingTarget(
        horizontalScrollController: horizontalScrollController,
        verticalScrollController: verticalScrollController,
      );
      if (_needsMoreSync(
        horizontalScrollController: horizontalScrollController,
        verticalScrollController: verticalScrollController,
      )) {
        _scheduleSync(
          horizontalScrollController: horizontalScrollController,
          verticalScrollController: verticalScrollController,
          isMounted: isMounted,
          isPinching: isPinching,
        );
      }
    });
  }

  bool _needsMoreSync({
    required ScrollController horizontalScrollController,
    required ScrollController verticalScrollController,
  }) {
    final target = _pendingTarget;
    if (target == null) {
      return false;
    }

    var needsMore = false;

    final horizontalPosition = resolveHorizontalScrollMetrics(
      controller: horizontalScrollController,
      contentWidth: _pendingContentWidth,
      fallbackViewportDimension: _pendingViewportWidth,
    ).position;
    if (horizontalPosition != null && horizontalPosition.hasContentDimensions) {
      final clampedHorizontal = target.horizontal.clamp(0.0, horizontalPosition.maxScrollExtent);
      if ((horizontalPosition.pixels - clampedHorizontal).abs() > pinchScrollJumpThreshold) {
        needsMore = true;
      }
    }

    final verticalPosition = resolveScrollPosition(verticalScrollController);
    if (verticalPosition != null && verticalPosition.hasContentDimensions) {
      final clampedVertical = target.vertical.clamp(0.0, verticalPosition.maxScrollExtent);
      if ((verticalPosition.pixels - clampedVertical).abs() > pinchScrollJumpThreshold) {
        needsMore = true;
      }
    }

    return needsMore;
  }
}

class PinchGesture implements InteractionGesture {
  PinchGesture({required PinchViewportGesture viewport}) : _viewport = viewport;

  final PinchViewportGesture _viewport;
  final Map<int, Offset> _activePointers = {};
  double? _pinchStartDistance;
  double _pinchStartZoom = 1;
  bool _isPinching = false;

  bool get isPinching => _isPinching;
  int get pointerCount => _activePointers.length;

  bool containsPointer(int pointer) => _activePointers.containsKey(pointer);
  Offset? pointerPosition(int pointer) => _activePointers[pointer];

  MapEntry<int, Offset>? get singlePointerEntry {
    if (_activePointers.length != 1) {
      return null;
    }
    return _activePointers.entries.first;
  }

  void addPointer(int pointer, Offset localPosition) {
    _activePointers[pointer] = localPosition;
  }

  void updatePointer(int pointer, Offset localPosition) {
    if (_activePointers.containsKey(pointer)) {
      _activePointers[pointer] = localPosition;
    }
  }

  void removePointer(int pointer) {
    _activePointers.remove(pointer);
  }

  @override
  void reset() {
    _activePointers.clear();
    _pinchStartDistance = null;
    _pinchStartZoom = 1;
    _isPinching = false;
    _viewport.reset();
  }

  bool beginIfNeeded({
    required bool isPaginated,
    required double currentZoom,
    required double Function(double localX) resolveLogicalX,
    required PinchPageResolver resolvePageAtPosition,
  }) {
    if (!isPaginated || _activePointers.length < 2) {
      return false;
    }

    final distance = _currentPinchDistance();
    if (distance == null || distance <= 0) {
      return false;
    }

    _pinchStartDistance = distance;
    _pinchStartZoom = currentZoom;
    _isPinching = true;
    _viewport.reset();

    final focal = _currentPinchFocal();
    if (focal != null) {
      final logicalX = resolveLogicalX(focal.dx);
      final (pageIdx, logicalY) = resolvePageAtPosition(focal.dy);
      _viewport.captureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);
    }

    return true;
  }

  void updateIfNeeded({
    required bool isPaginated,
    required Layout? layout,
    required double viewportWidth,
    required double currentZoom,
    required double Function(double localX) resolveLogicalX,
    required PinchPageResolver resolvePageAtPosition,
    required void Function(double zoom, {bool commitRender}) setZoom,
    required ContentGeometry Function(double zoom) geometryBuilder,
    required ScrollController horizontalScrollController,
    required ScrollController verticalScrollController,
    required bool Function() isMounted,
    void Function(double previousZoom, double nextZoom)? onZoomChanged,
  }) {
    if (!_isPinching || !isPaginated) {
      return;
    }
    if (layout is! PaginatedLayout) {
      return;
    }

    final startDistance = _pinchStartDistance;
    if (startDistance == null || startDistance <= 0) {
      return;
    }

    final distance = _currentPinchDistance();
    if (distance == null || distance <= 0) {
      return;
    }

    final focal = _currentPinchFocal();
    if (focal == null) {
      return;
    }

    final logicalX = resolveLogicalX(focal.dx);
    final (pageIdx, logicalY) = resolvePageAtPosition(focal.dy);
    _viewport.ensureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);

    final nextZoom = clampPaginatedZoom(
      zoom: _pinchStartZoom * (distance / startDistance),
      pageWidth: layout.pageWidth,
      viewportWidth: viewportWidth,
    );
    final zoomForSync = zoomEquals(nextZoom, currentZoom) ? currentZoom : nextZoom;
    if (zoomDiffers(nextZoom, currentZoom)) {
      onZoomChanged?.call(currentZoom, nextZoom);
      setZoom(nextZoom);
    }

    _viewport.syncViewport(
      focal: focal,
      geometry: geometryBuilder(zoomForSync),
      viewportWidth: viewportWidth,
      horizontalScrollController: horizontalScrollController,
      verticalScrollController: verticalScrollController,
      isMounted: isMounted,
      isPinching: () => _isPinching,
    );
  }

  void endIfNeeded({required double currentZoom, required void Function(double zoom, {bool commitRender}) setZoom}) {
    if (!_isPinching) {
      return;
    }
    _isPinching = false;
    _pinchStartDistance = null;
    _viewport.reset();
    setZoom(currentZoom, commitRender: true);
  }

  double? _currentPinchDistance() {
    if (_activePointers.length < 2) {
      return null;
    }

    final points = _activePointers.values.toList(growable: false);
    if (points.length < 2) {
      return null;
    }
    return (points[0] - points[1]).distance;
  }

  Offset? _currentPinchFocal() {
    if (_activePointers.length < 2) {
      return null;
    }

    final points = _activePointers.values.toList(growable: false);
    if (points.length < 2) {
      return null;
    }
    return (points[0] + points[1]) / 2;
  }
}

extension PinchGestureMethods on EditorInteractionController {
  String? _zoomSnapKey(double value) {
    final layout = scope.controller.state.layout;
    final viewWidth = readViewWidth();
    if (layout is! PaginatedLayout || viewWidth <= 0) {
      return null;
    }

    final fitWidthZoom = computePaginatedFitWidthZoom(pageWidth: layout.pageWidth, viewportWidth: viewWidth);
    final unitZoom = clampDocumentZoom(1, bounds: computePaginatedZoomBounds(pageWidth: layout.pageWidth));

    if (zoomEquals(value, fitWidthZoom)) {
      return 'fit-width';
    }
    if (zoomEquals(value, unitZoom)) {
      return 'unit';
    }
    return null;
  }

  void _maybeSendZoomSnapHaptic({required double previousZoom, required double nextZoom}) {
    if (zoomEquals(previousZoom, nextZoom)) {
      return;
    }

    final nextSnap = _zoomSnapKey(nextZoom);
    if (nextSnap == null) {
      return;
    }

    final previousSnap = _zoomSnapKey(previousZoom);
    if (previousSnap == nextSnap) {
      return;
    }

    unawaited(HapticFeedback.selectionClick());
  }

  void _beginPinchIfNeeded() {
    final geometry = readGeometry();
    final started = pinchGesture.beginIfNeeded(
      isPaginated: geometry.isPaginated,
      currentZoom: scope.displayZoom.value,
      resolveLogicalX: _resolvePointerX,
      resolvePageAtPosition: getPageAtPosition,
    );
    if (!started) {
      return;
    }

    _applyTransition(InteractionEvent.pinchStart);
  }

  void _updatePinchZoom() {
    final geometry = readGeometry();
    pinchGesture.updateIfNeeded(
      isPaginated: geometry.isPaginated,
      layout: scope.controller.state.layout,
      viewportWidth: readViewWidth(),
      currentZoom: scope.displayZoom.value,
      resolveLogicalX: _resolvePointerX,
      resolvePageAtPosition: getPageAtPosition,
      setZoom: scope.setZoom,
      geometryBuilder: (nextZoom) => ContentGeometry(
        layout: scope.controller.state.layout!,
        pages: scope.controller.state.pages,
        titleAreaHeight: scope.titleAreaHeight.value,
        selection: scope.controller.state.selection,
        zoom: nextZoom,
      ),
      horizontalScrollController: scope.horizontalScrollController,
      verticalScrollController: scope.verticalScrollController,
      isMounted: () => context.mounted,
      onZoomChanged: (previousZoom, nextZoom) {
        _maybeSendZoomSnapHaptic(previousZoom: previousZoom, nextZoom: nextZoom);
      },
    );
  }

  void _endPinchIfNeeded() {
    pinchGesture.endIfNeeded(currentZoom: scope.displayZoom.value, setZoom: scope.setZoom);
    _applyTransition(InteractionEvent.pinchEnd);
  }

  bool _handlePointerZoom(PointerScrollEvent event, Set<LogicalKeyboardKey> keysPressed) {
    final geometry = readGeometry();
    if (!geometry.isPaginated) {
      return false;
    }

    final isZoomModifierPressed =
        keysPressed.contains(LogicalKeyboardKey.controlLeft) ||
        keysPressed.contains(LogicalKeyboardKey.controlRight) ||
        keysPressed.contains(LogicalKeyboardKey.metaLeft) ||
        keysPressed.contains(LogicalKeyboardKey.metaRight);
    if (!isZoomModifierPressed) {
      return false;
    }

    final layout = scope.controller.state.layout;
    if (layout is! PaginatedLayout) {
      return false;
    }

    final zoomDelta = event.scrollDelta.dy.abs() >= event.scrollDelta.dx.abs()
        ? event.scrollDelta.dy
        : event.scrollDelta.dx;
    if (zoomDelta == 0) {
      return true;
    }

    final currentZoom = scope.displayZoom.value;
    final nextZoom = clampPaginatedZoom(
      zoom: currentZoom * math.exp(-zoomDelta / 240),
      pageWidth: layout.pageWidth,
      viewportWidth: readViewWidth(),
    );
    if (zoomEquals(nextZoom, currentZoom)) {
      return true;
    }

    final focal = event.localPosition;
    final logicalX = _resolvePointerX(focal.dx);
    final (pageIdx, logicalY) = getPageAtPosition(focal.dy);
    pinchViewportGesture.captureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);

    _maybeSendZoomSnapHaptic(previousZoom: currentZoom, nextZoom: nextZoom);
    scope.setZoom(nextZoom);

    pinchViewportGesture.syncViewport(
      focal: focal,
      geometry: ContentGeometry(
        layout: layout,
        pages: scope.controller.state.pages,
        titleAreaHeight: scope.titleAreaHeight.value,
        selection: scope.controller.state.selection,
        zoom: nextZoom,
      ),
      viewportWidth: readViewWidth(),
      horizontalScrollController: scope.horizontalScrollController,
      verticalScrollController: scope.verticalScrollController,
      isMounted: () => context.mounted,
      isPinching: () => true,
    );
    return true;
  }
}
