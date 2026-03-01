import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

class PinchZoomSession {
  PinchAnchor? _anchor;
  PinchScrollTarget? _pendingTarget;
  double _pendingContentWidth = 0;
  double _pendingViewportWidth = 0;
  bool _syncScheduled = false;

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

class PinchGestureController {
  final PinchZoomSession _session = PinchZoomSession();
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

  void reset() {
    _activePointers.clear();
    _pinchStartDistance = null;
    _pinchStartZoom = 1;
    _isPinching = false;
    _session.reset();
  }

  bool beginIfNeeded({
    required bool isPaginated,
    required double currentZoom,
    required double Function(double localX) resolveLogicalX,
    required (int pageIdx, double localY) Function(double y) resolvePageAtPosition,
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
    _session.reset();

    final focal = _currentPinchFocal();
    if (focal != null) {
      final logicalX = resolveLogicalX(focal.dx);
      final (pageIdx, logicalY) = resolvePageAtPosition(focal.dy);
      _session.captureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);
    }

    return true;
  }

  void updateIfNeeded({
    required bool isPaginated,
    required Layout? layout,
    required double viewportWidth,
    required double currentZoom,
    required double Function(double localX) resolveLogicalX,
    required (int pageIdx, double localY) Function(double y) resolvePageAtPosition,
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
    _session.ensureAnchor(pageIdx: pageIdx, logicalX: logicalX, logicalY: logicalY);

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

    _session.syncViewport(
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
    _session.reset();
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
