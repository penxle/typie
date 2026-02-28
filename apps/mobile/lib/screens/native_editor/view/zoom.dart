import 'dart:math' as math;

import 'package:flutter/widgets.dart';

const minDocumentDisplayWidth = 100.0;
const maxDocumentZoom = 2.0;
const fitWidthZoomSnapThreshold = 0.02;
const unitZoomSnapThreshold = 0.02;
const renderZoomDebounce = Duration(milliseconds: 120);
const zoomEpsilon = 0.0001;
const pinchScrollJumpThreshold = 0.5;
const horizontalScrollExtentThreshold = 0.5;
const _horizontalSelectionTieThreshold = 0.5;
const _horizontalPixelTieThreshold = 0.01;
const _horizontalNearExtentThreshold = 1.0;
const _horizontalViewportWeight = 0.35;

typedef PinchAnchor = ({int pageIdx, double logicalX, double logicalY});
typedef PinchScrollTarget = ({double horizontal, double vertical});
typedef ZoomBounds = ({double min, double max});
typedef HorizontalScrollMetrics = ({
  ScrollPosition? position,
  double viewportDimension,
  double scrollOffset,
  double maxScrollExtent,
  double expectedMaxScrollExtent,
});

bool hasScrollableExtent(double extent) {
  return extent.isFinite && extent > horizontalScrollExtentThreshold;
}

extension HorizontalScrollMetricsX on HorizontalScrollMetrics {
  bool get hasResolvedPosition {
    final current = position;
    return current != null && current.hasContentDimensions;
  }

  bool get expectsScrollableContent => hasScrollableExtent(expectedMaxScrollExtent);

  bool get hasScrollablePositionExtent => hasScrollableExtent(maxScrollExtent);

  bool get canScrollHorizontally => hasResolvedPosition && expectsScrollableContent && hasScrollablePositionExtent;

  ScrollPosition? get activePosition => canScrollHorizontally ? position : null;
}

final _preferredHorizontalPositionByController = Expando<ScrollPosition>('preferredHorizontalPosition');

void setPreferredHorizontalScrollPosition(ScrollController controller, ScrollPosition position) {
  if (!controller.hasClients) {
    return;
  }
  if (controller.positions.contains(position)) {
    _preferredHorizontalPositionByController[controller] = position;
  }
}

void clearPreferredHorizontalScrollPosition(ScrollController controller, ScrollPosition position) {
  final cached = _preferredHorizontalPositionByController[controller];
  if (identical(cached, position)) {
    _preferredHorizontalPositionByController[controller] = null;
  }
}

ZoomBounds computePaginatedZoomBounds({required double pageWidth, double minDisplayWidth = minDocumentDisplayWidth}) {
  final safePageWidth = pageWidth.isFinite && pageWidth > 0 ? pageWidth : 1.0;

  final minZoom = (minDisplayWidth / safePageWidth).clamp(0.01, double.infinity);
  final maxZoom = maxDocumentZoom.clamp(minZoom, double.infinity);

  return (min: minZoom, max: maxZoom);
}

double clampDocumentZoom(double zoom, {required ZoomBounds bounds}) {
  if (!zoom.isFinite) {
    return bounds.min;
  }
  return zoom.clamp(bounds.min, bounds.max);
}

double computePaginatedFitWidthZoom({required double pageWidth, required double viewportWidth}) {
  final bounds = computePaginatedZoomBounds(pageWidth: pageWidth);
  final safePageWidth = pageWidth.isFinite && pageWidth > 0 ? pageWidth : 1.0;
  final safeViewportWidth = viewportWidth.isFinite && viewportWidth > 0 ? viewportWidth : safePageWidth;
  return (safeViewportWidth / safePageWidth).clamp(bounds.min, bounds.max);
}

double computeInitialPaginatedZoom({required double pageWidth, required double viewportWidth}) {
  final fitWidthZoom = computePaginatedFitWidthZoom(pageWidth: pageWidth, viewportWidth: viewportWidth);
  return math.min(fitWidthZoom, 1);
}

double clampPaginatedZoom({required double zoom, required double pageWidth, required double viewportWidth}) {
  final bounds = computePaginatedZoomBounds(pageWidth: pageWidth);
  final clamped = clampDocumentZoom(zoom, bounds: bounds);
  final fitWidthZoom = computePaginatedFitWidthZoom(pageWidth: pageWidth, viewportWidth: viewportWidth);
  final unitZoom = clampDocumentZoom(1, bounds: bounds);

  double? snapped;
  var bestDistance = double.infinity;

  final fitWidthDistance = (clamped - fitWidthZoom).abs();
  if (fitWidthDistance <= fitWidthZoomSnapThreshold) {
    snapped = fitWidthZoom;
    bestDistance = fitWidthDistance;
  }

  final unitDistance = (clamped - unitZoom).abs();
  if (unitDistance <= unitZoomSnapThreshold && unitDistance < bestDistance) {
    snapped = unitZoom;
  }

  return snapped ?? clamped;
}

double renderZoomForDisplay(double displayZoom) {
  if (!displayZoom.isFinite) {
    return 1;
  }
  return displayZoom <= 0 ? 0.01 : displayZoom;
}

bool zoomEquals(double a, double b) => (a - b).abs() < zoomEpsilon;
bool zoomDiffers(double a, double b) => !zoomEquals(a, b);
bool isUnitZoom(double zoom) => zoomEquals(zoom, 1);

double computeExpectedScrollExtent({required double contentExtent, required double viewportExtent}) {
  final safeContentExtent = contentExtent.isFinite ? contentExtent : 0.0;
  final safeViewportExtent = viewportExtent.isFinite ? viewportExtent : 0.0;
  final expected = safeContentExtent - safeViewportExtent;
  return expected > 0 ? expected : 0.0;
}

double resolveScrollOffset(ScrollController controller) {
  return resolveScrollPosition(controller)?.pixels ?? 0.0;
}

ScrollPosition? resolveScrollPosition(ScrollController controller) {
  if (!controller.hasClients) {
    return null;
  }

  final positions = controller.positions.toList(growable: false);
  if (positions.isEmpty) {
    return null;
  }

  ScrollPosition best = positions.first;

  for (final position in positions.skip(1)) {
    final bestHasDims = best.hasContentDimensions;
    final nextHasDims = position.hasContentDimensions;

    if (bestHasDims != nextHasDims) {
      if (nextHasDims) {
        best = position;
      }
      continue;
    }

    if (!nextHasDims) {
      continue;
    }

    final bestScore = best.maxScrollExtent + best.viewportDimension * 0.001;
    final nextScore = position.maxScrollExtent + position.viewportDimension * 0.001;
    if (nextScore > bestScore) {
      best = position;
    }
  }

  return best;
}

HorizontalScrollMetrics resolveHorizontalScrollMetrics({
  required ScrollController controller,
  required double contentWidth,
  required double fallbackViewportDimension,
}) {
  final unresolved = _unresolvedHorizontalMetrics(
    viewportDimension: fallbackViewportDimension,
    contentWidth: contentWidth,
  );
  if (!controller.hasClients) {
    _preferredHorizontalPositionByController[controller] = null;
    return unresolved;
  }

  final positions = controller.positions.where((position) => position.hasContentDimensions).toList(growable: false);
  if (positions.isEmpty) {
    final fallbackPosition = resolveScrollPosition(controller);
    if (fallbackPosition != null) {
      _preferredHorizontalPositionByController[controller] = fallbackPosition;
      return _resolvedHorizontalMetrics(position: fallbackPosition, contentWidth: contentWidth);
    }
    return unresolved;
  }

  if (positions.length == 1) {
    final position = positions.first;
    _preferredHorizontalPositionByController[controller] = position;
    return _resolvedHorizontalMetrics(position: position, contentWidth: contentWidth);
  }

  final targetViewportDimension = _resolveTargetViewportDimension(
    fallbackViewportDimension: fallbackViewportDimension,
    positions: positions,
  );
  final targetExpectedMaxScrollExtent = computeExpectedScrollExtent(
    contentExtent: contentWidth,
    viewportExtent: targetViewportDimension,
  );

  final selectionPool = _selectHorizontalCandidatePool(
    positions: positions,
    targetExpectedMaxScrollExtent: targetExpectedMaxScrollExtent,
  );
  final cached = _preferredHorizontalPositionByController[controller];
  final best = _selectBestHorizontalPosition(
    selectionPool: selectionPool,
    targetViewportDimension: targetViewportDimension,
    targetExpectedMaxScrollExtent: targetExpectedMaxScrollExtent,
    cached: cached,
  );
  _preferredHorizontalPositionByController[controller] = best;
  return _resolvedHorizontalMetrics(
    position: best,
    contentWidth: contentWidth,
    expectedViewportDimension: targetViewportDimension,
  );
}

HorizontalScrollMetrics _unresolvedHorizontalMetrics({
  required double viewportDimension,
  required double contentWidth,
}) {
  final safeViewportDimension = _normalizeViewportDimension(viewportDimension);
  final expected = computeExpectedScrollExtent(contentExtent: contentWidth, viewportExtent: safeViewportDimension);
  return (
    position: null,
    viewportDimension: safeViewportDimension,
    scrollOffset: 0.0,
    maxScrollExtent: expected,
    expectedMaxScrollExtent: expected,
  );
}

HorizontalScrollMetrics _resolvedHorizontalMetrics({
  required ScrollPosition position,
  required double contentWidth,
  double? expectedViewportDimension,
}) {
  final expected = computeExpectedScrollExtent(
    contentExtent: contentWidth,
    viewportExtent: expectedViewportDimension ?? position.viewportDimension,
  );
  return (
    position: position,
    viewportDimension: position.viewportDimension,
    scrollOffset: position.pixels,
    maxScrollExtent: position.maxScrollExtent,
    expectedMaxScrollExtent: expected,
  );
}

double _normalizeViewportDimension(double viewportDimension) {
  return (viewportDimension.isFinite && viewportDimension >= 0) ? viewportDimension : 0.0;
}

double _resolveTargetViewportDimension({
  required double fallbackViewportDimension,
  required List<ScrollPosition> positions,
}) {
  if (fallbackViewportDimension.isFinite && fallbackViewportDimension > 0) {
    return fallbackViewportDimension;
  }
  return _inferSmallestViewportDimension(positions);
}

double _inferSmallestViewportDimension(List<ScrollPosition> positions) {
  var inferred = double.infinity;
  for (final position in positions) {
    final viewport = position.viewportDimension;
    if (viewport.isFinite && viewport > 0 && viewport < inferred) {
      inferred = viewport;
    }
  }
  if (inferred.isFinite) {
    return inferred;
  }
  return positions.first.viewportDimension;
}

List<ScrollPosition> _selectHorizontalCandidatePool({
  required List<ScrollPosition> positions,
  required double targetExpectedMaxScrollExtent,
}) {
  if (!hasScrollableExtent(targetExpectedMaxScrollExtent)) {
    return positions;
  }
  final scrollable = positions
      .where((position) => hasScrollableExtent(position.maxScrollExtent))
      .toList(growable: false);
  return scrollable.isNotEmpty ? scrollable : positions;
}

double _scoreHorizontalCandidate({
  required ScrollPosition position,
  required double targetViewportDimension,
  required double targetExpectedMaxScrollExtent,
}) {
  final extentDelta = (position.maxScrollExtent - targetExpectedMaxScrollExtent).abs();
  final viewportDelta = (position.viewportDimension - targetViewportDimension).abs();
  final staleNoScrollPenalty =
      hasScrollableExtent(targetExpectedMaxScrollExtent) && !hasScrollableExtent(position.maxScrollExtent)
      ? targetExpectedMaxScrollExtent + 120.0
      : 0.0;
  return extentDelta + viewportDelta * _horizontalViewportWeight + staleNoScrollPenalty;
}

ScrollPosition _selectBestHorizontalPosition({
  required List<ScrollPosition> selectionPool,
  required double targetViewportDimension,
  required double targetExpectedMaxScrollExtent,
  required ScrollPosition? cached,
}) {
  final hasCached = cached != null && selectionPool.contains(cached);
  final cachedPosition = hasCached ? cached : null;
  var best = cachedPosition ?? selectionPool.first;
  var bestScore = _scoreHorizontalCandidate(
    position: best,
    targetViewportDimension: targetViewportDimension,
    targetExpectedMaxScrollExtent: targetExpectedMaxScrollExtent,
  );

  for (final position in selectionPool.skip(1)) {
    if (identical(position, best)) {
      continue;
    }

    final score = _scoreHorizontalCandidate(
      position: position,
      targetViewportDimension: targetViewportDimension,
      targetExpectedMaxScrollExtent: targetExpectedMaxScrollExtent,
    );
    final significantlyBetter = score < bestScore - _horizontalSelectionTieThreshold;
    final nearTie = (score - bestScore).abs() <= _horizontalSelectionTieThreshold;

    if (significantlyBetter) {
      best = position;
      bestScore = score;
      continue;
    }

    if (!nearTie) {
      continue;
    }

    final bestIsScrolling = best.isScrollingNotifier.value;
    final positionIsScrolling = position.isScrollingNotifier.value;
    if (positionIsScrolling != bestIsScrolling) {
      if (positionIsScrolling) {
        best = position;
        bestScore = score;
      }
      continue;
    }

    if (hasCached && identical(best, cached)) {
      continue;
    }

    final bestExtentDelta = (best.maxScrollExtent - targetExpectedMaxScrollExtent).abs();
    final nextExtentDelta = (position.maxScrollExtent - targetExpectedMaxScrollExtent).abs();
    final bothNearExtent =
        bestExtentDelta <= _horizontalNearExtentThreshold && nextExtentDelta <= _horizontalNearExtentThreshold;
    final preferSmallerViewport =
        bothNearExtent && position.viewportDimension + _horizontalPixelTieThreshold < best.viewportDimension;
    final preferLargerMaxWhenScrollable =
        hasScrollableExtent(targetExpectedMaxScrollExtent) &&
        bothNearExtent &&
        position.maxScrollExtent > best.maxScrollExtent + horizontalScrollExtentThreshold;

    if (preferLargerMaxWhenScrollable ||
        preferSmallerViewport ||
        position.pixels.abs() > best.pixels.abs() + _horizontalPixelTieThreshold ||
        ((position.pixels.abs() - best.pixels.abs()).abs() <= _horizontalPixelTieThreshold &&
            position.viewportDimension > best.viewportDimension + _horizontalPixelTieThreshold)) {
      best = position;
      bestScore = score;
    }
  }

  return best;
}
