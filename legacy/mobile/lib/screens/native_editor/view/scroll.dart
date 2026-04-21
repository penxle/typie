import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

const _scrollMargin = 60.0;

bool scrollToCursor({
  required ScrollController verticalController,
  required ScrollController horizontalController,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  bool typewriterEnabled = false,
  double typewriterPosition = 0.5,
  double viewportTopInset = 0,
  double viewportBottomInset = 0,
}) {
  final didScrollVertical = _scrollVertical(
    controller: verticalController,
    geometry: geometry,
    cursor: cursor,
    typewriterEnabled: typewriterEnabled,
    typewriterPosition: typewriterPosition,
    viewportTopInset: viewportTopInset,
    viewportBottomInset: viewportBottomInset,
  );
  final didScrollHorizontal = _scrollHorizontal(
    controller: horizontalController,
    geometry: geometry,
    cursor: cursor,
    animate: !typewriterEnabled,
  );

  return didScrollVertical || didScrollHorizontal;
}

bool _scrollVertical({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  required bool typewriterEnabled,
  required double typewriterPosition,
  required double viewportTopInset,
  required double viewportBottomInset,
}) {
  final position = resolveScrollPosition(controller);
  if (position == null || !position.hasContentDimensions) {
    return false;
  }

  final cursorGlobalY = geometry.cursorTopInContent(cursor);
  final viewportHeight = position.viewportDimension;
  final cursorHeight = geometry.toDisplayY(cursor.height);

  if (typewriterEnabled) {
    final usableViewportHeight = math.max(0, viewportHeight - viewportTopInset - viewportBottomInset);
    final availableRange = math.max(0, usableViewportHeight - cursorHeight);
    final targetScroll = cursorGlobalY - viewportTopInset - availableRange * typewriterPosition;
    final totalContentHeight = geometry.totalContentHeight(
      viewportHeight: viewportHeight,
      cursor: cursor,
      typewriterEnabled: true,
      typewriterPosition: typewriterPosition,
      viewportTopInset: viewportTopInset,
      viewportBottomInset: viewportBottomInset,
      additionalBottomPadding: viewportBottomInset,
    );
    final maxScrollExtent = math.max<double>(0, totalContentHeight - viewportHeight);

    final clampedTarget = targetScroll.clamp(0.0, maxScrollExtent);
    final distance = (position.pixels - clampedTarget).abs();
    if (distance <= 1) {
      return false;
    }

    position.jumpTo(clampedTarget);
    return true;
  }

  return _jumpToKeepCursorInScrollMargin(
    position: position,
    cursorTop: cursorGlobalY,
    cursorHeight: cursorHeight,
    viewportHeight: viewportHeight,
    viewportTopInset: viewportTopInset,
    viewportBottomInset: viewportBottomInset,
    maxScrollExtent: position.maxScrollExtent,
  );
}

bool _jumpToKeepCursorInScrollMargin({
  required ScrollPosition position,
  required double cursorTop,
  required double cursorHeight,
  required double viewportHeight,
  required double viewportTopInset,
  required double viewportBottomInset,
  required double maxScrollExtent,
}) {
  final scrollOffset = position.pixels;
  final cursorBottom = cursorTop + cursorHeight;
  final visibleTop = scrollOffset + viewportTopInset;
  final visibleBottom = scrollOffset + viewportHeight - viewportBottomInset;

  if (cursorBottom > visibleBottom - _scrollMargin) {
    final target = (cursorBottom - viewportHeight + viewportBottomInset + _scrollMargin).clamp(0.0, maxScrollExtent);
    if ((target - scrollOffset).abs() <= 1) {
      return false;
    }
    position.jumpTo(target);
    return true;
  } else if (cursorTop < visibleTop + _scrollMargin) {
    final target = (cursorTop - viewportTopInset - _scrollMargin).clamp(0.0, maxScrollExtent);
    if ((target - scrollOffset).abs() <= 1) {
      return false;
    }
    position.jumpTo(target);
    return true;
  }

  return false;
}

bool _scrollHorizontal({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  bool animate = true,
}) {
  final horizontalMetrics = resolveHorizontalScrollMetrics(
    controller: controller,
    contentWidth: geometry.contentWidth,
    fallbackViewportDimension: geometry.contentWidth,
  );
  final position = horizontalMetrics.activePosition;
  if (!horizontalMetrics.canScrollHorizontally || position == null) {
    return false;
  }

  const scrollMargin = _scrollMargin;
  final cursorX = geometry.toDisplayX(cursor.x) + geometry.horizontalPadding;
  final scrollOffset = horizontalMetrics.scrollOffset;
  final viewportWidth = horizontalMetrics.viewportDimension;
  final cursorRight = cursorX + geometry.toDisplayX(2);

  if (cursorRight > scrollOffset + viewportWidth - scrollMargin) {
    final target = (cursorRight - viewportWidth + scrollMargin).clamp(0.0, position.maxScrollExtent);
    if ((target - scrollOffset).abs() <= 1) {
      return false;
    }
    if (animate) {
      unawaited(position.animateTo(target, duration: const Duration(milliseconds: 100), curve: Curves.easeOut));
    } else {
      position.jumpTo(target);
    }
    return true;
  } else if (cursorX < scrollOffset + scrollMargin) {
    final target = (cursorX - scrollMargin).clamp(0.0, position.maxScrollExtent);
    if ((target - scrollOffset).abs() <= 1) {
      return false;
    }
    if (animate) {
      unawaited(position.animateTo(target, duration: const Duration(milliseconds: 100), curve: Curves.easeOut));
    } else {
      position.jumpTo(target);
    }
    return true;
  }

  return false;
}

void scrollToOverlayTarget({
  required ScrollController verticalScrollController,
  required ScrollController horizontalScrollController,
  required ContentGeometry geometry,
  required int pageIdx,
  required double targetX,
  required double targetY,
  required double targetWidth,
  double targetHeight = 0,
  double targetAnchor = 0.5,
  double viewportAnchor = 0.5,
  double viewportTopInset = 0,
  double viewportBottomInset = 0,
}) {
  final offsets = geometry.computeCumulativePageOffsets();
  final absoluteY = geometry.titleAreaHeight + offsets[pageIdx] + geometry.toDisplayY(targetY);

  final verticalPosition = resolveScrollPosition(verticalScrollController);
  if (verticalPosition != null && verticalPosition.hasContentDimensions) {
    final clampedTargetAnchor = targetAnchor.clamp(0.0, 1.0);
    final clampedViewportAnchor = viewportAnchor.clamp(0.0, 1.0);
    final viewportHeight = verticalPosition.viewportDimension;
    final usableViewportHeight = math.max(0, viewportHeight - viewportTopInset - viewportBottomInset);
    final absoluteAnchorY = absoluteY + geometry.toDisplayY(targetHeight) * clampedTargetAnchor;
    final targetOffset = (absoluteAnchorY - viewportTopInset - usableViewportHeight * clampedViewportAnchor).clamp(
      0.0,
      verticalPosition.maxScrollExtent,
    );
    final currentOffset = verticalPosition.pixels;
    if ((targetOffset - currentOffset).abs() > 1) {
      unawaited(
        verticalPosition.animateTo(targetOffset, duration: const Duration(milliseconds: 200), curve: Curves.easeOut),
      );
    }
  }

  final horizontalMetrics = resolveHorizontalScrollMetrics(
    controller: horizontalScrollController,
    contentWidth: geometry.contentWidth,
    fallbackViewportDimension: geometry.contentWidth,
  );
  final horizontalPosition = horizontalMetrics.activePosition;
  if (!horizontalMetrics.canScrollHorizontally || horizontalPosition == null) {
    return;
  }

  const scrollMargin = 60.0;
  final matchX = geometry.toDisplayX(targetX) + geometry.horizontalPadding;
  final matchRight = matchX + geometry.toDisplayX(targetWidth);
  final scrollOffset = horizontalMetrics.scrollOffset;
  final viewportWidth = horizontalMetrics.viewportDimension;

  if (matchRight > scrollOffset + viewportWidth - scrollMargin) {
    final target = (matchRight - viewportWidth + scrollMargin).clamp(0.0, horizontalPosition.maxScrollExtent);
    if ((target - scrollOffset).abs() > 1) {
      unawaited(
        horizontalPosition.animateTo(target, duration: const Duration(milliseconds: 200), curve: Curves.easeOut),
      );
    }
  } else if (matchX < scrollOffset + scrollMargin) {
    final target = (matchX - scrollMargin).clamp(0.0, horizontalPosition.maxScrollExtent);
    if ((target - scrollOffset).abs() > 1) {
      unawaited(
        horizontalPosition.animateTo(target, duration: const Duration(milliseconds: 200), curve: Curves.easeOut),
      );
    }
  }
}
