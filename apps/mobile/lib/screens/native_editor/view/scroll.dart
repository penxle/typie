import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

const _scrollMargin = 60.0;

void scrollToCursor({
  required ScrollController verticalController,
  required ScrollController horizontalController,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  bool typewriterEnabled = false,
  double typewriterPosition = 0.5,
}) {
  _scrollVertical(
    controller: verticalController,
    geometry: geometry,
    cursor: cursor,
    typewriterEnabled: typewriterEnabled,
    typewriterPosition: typewriterPosition,
  );
  _scrollHorizontal(controller: horizontalController, geometry: geometry, cursor: cursor, animate: !typewriterEnabled);
}

void _scrollVertical({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  required bool typewriterEnabled,
  required double typewriterPosition,
}) {
  final position = resolveScrollPosition(controller);
  if (position == null || !position.hasContentDimensions) {
    return;
  }

  final cursorGlobalY = geometry.cursorTopInContent(cursor);
  final viewportHeight = position.viewportDimension;
  final cursorHeight = geometry.toDisplayY(cursor.height);

  if (typewriterEnabled) {
    final availableRange = viewportHeight - cursorHeight;
    final targetScroll = cursorGlobalY - availableRange * typewriterPosition;
    final totalContentHeight = geometry.totalContentHeight(
      viewportHeight: viewportHeight,
      cursor: cursor,
      typewriterEnabled: true,
      typewriterPosition: typewriterPosition,
    );
    final maxScrollExtent = math.max<double>(0, totalContentHeight - viewportHeight);

    final clampedTarget = targetScroll.clamp(0.0, maxScrollExtent);
    final distance = (position.pixels - clampedTarget).abs();
    if (distance <= 1) {
      return;
    }

    position.jumpTo(clampedTarget);
    return;
  }

  _jumpToKeepCursorInScrollMargin(
    position: position,
    cursorTop: cursorGlobalY,
    cursorHeight: cursorHeight,
    viewportHeight: viewportHeight,
    maxScrollExtent: position.maxScrollExtent,
  );
}

void _jumpToKeepCursorInScrollMargin({
  required ScrollPosition position,
  required double cursorTop,
  required double cursorHeight,
  required double viewportHeight,
  required double maxScrollExtent,
}) {
  final scrollOffset = position.pixels;
  final cursorBottom = cursorTop + cursorHeight;

  if (cursorBottom > scrollOffset + viewportHeight - _scrollMargin) {
    position.jumpTo((cursorBottom - viewportHeight + _scrollMargin).clamp(0.0, maxScrollExtent));
  } else if (cursorTop < scrollOffset + _scrollMargin) {
    position.jumpTo((cursorTop - _scrollMargin).clamp(0.0, maxScrollExtent));
  }
}

void _scrollHorizontal({
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
    return;
  }

  const scrollMargin = _scrollMargin;
  final cursorX = geometry.toDisplayX(cursor.x) + geometry.horizontalPadding;
  final scrollOffset = horizontalMetrics.scrollOffset;
  final viewportWidth = horizontalMetrics.viewportDimension;
  final cursorRight = cursorX + geometry.toDisplayX(2);

  if (cursorRight > scrollOffset + viewportWidth - scrollMargin) {
    final target = (cursorRight - viewportWidth + scrollMargin).clamp(0.0, position.maxScrollExtent);
    if (animate) {
      unawaited(position.animateTo(target, duration: const Duration(milliseconds: 100), curve: Curves.easeOut));
    } else {
      position.jumpTo(target);
    }
  } else if (cursorX < scrollOffset + scrollMargin) {
    final target = (cursorX - scrollMargin).clamp(0.0, position.maxScrollExtent);
    if (animate) {
      unawaited(position.animateTo(target, duration: const Duration(milliseconds: 100), curve: Curves.easeOut));
    } else {
      position.jumpTo(target);
    }
  }
}

void scrollToOverlayTarget({
  required ScrollController verticalScrollController,
  required ScrollController horizontalScrollController,
  required ContentGeometry geometry,
  required int pageIdx,
  required double targetX,
  required double targetY,
  required double targetWidth,
}) {
  final offsets = geometry.computeCumulativePageOffsets();
  final absoluteY = geometry.titleAreaHeight + offsets[pageIdx] + geometry.toDisplayY(targetY);

  final verticalPosition = resolveScrollPosition(verticalScrollController);
  if (verticalPosition != null && verticalPosition.hasContentDimensions) {
    final viewportHeight = verticalPosition.viewportDimension;
    final targetOffset = (absoluteY - viewportHeight / 3).clamp(0.0, verticalPosition.maxScrollExtent);
    unawaited(
      verticalPosition.animateTo(targetOffset, duration: const Duration(milliseconds: 200), curve: Curves.easeOut),
    );
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
    unawaited(
      horizontalPosition.animateTo(
        (matchRight - viewportWidth + scrollMargin).clamp(0.0, horizontalPosition.maxScrollExtent),
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      ),
    );
  } else if (matchX < scrollOffset + scrollMargin) {
    unawaited(
      horizontalPosition.animateTo(
        (matchX - scrollMargin).clamp(0.0, horizontalPosition.maxScrollExtent),
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      ),
    );
  }
}
