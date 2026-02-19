import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';

const _scrollMargin = 60.0;

extension SafeScrollAccess on ScrollController {
  /// [position]/[offset] 접근 전에 사용. 여러 scroll view에 attach된 경우 false 반환.
  bool get hasSingleClient => hasClients && positions.length == 1;
}

void scrollToCursor({
  required ScrollController verticalController,
  required ScrollController horizontalController,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  bool typewriterEnabled = false,
  double typewriterPosition = 0.5,
  bool typewriterAnimate = true,
}) {
  _scrollVertical(
    controller: verticalController,
    geometry: geometry,
    cursor: cursor,
    typewriterEnabled: typewriterEnabled,
    typewriterPosition: typewriterPosition,
    typewriterAnimate: typewriterAnimate,
  );

  if (typewriterEnabled && !typewriterAnimate) {
    return;
  }

  _scrollHorizontal(controller: horizontalController, geometry: geometry, cursor: cursor);
}

void _scrollVertical({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  required bool typewriterEnabled,
  required double typewriterPosition,
  required bool typewriterAnimate,
}) {
  if (!controller.hasSingleClient) {
    return;
  }

  final cursorGlobalY = geometry.cursorTopInContent(cursor);
  final viewportHeight = controller.position.viewportDimension;

  if (typewriterEnabled) {
    final availableRange = viewportHeight - cursor.height;
    final targetScroll = cursorGlobalY - availableRange * typewriterPosition;
    final totalContentHeight = geometry.totalContentHeight(
      viewportHeight: viewportHeight,
      cursor: cursor,
      typewriterEnabled: true,
      typewriterPosition: typewriterPosition,
    );
    final maxScrollExtent = math.max<double>(0, totalContentHeight - viewportHeight);

    _jumpToKeepCursorInScrollMargin(
      controller: controller,
      cursorTop: cursorGlobalY,
      cursorHeight: cursor.height,
      viewportHeight: viewportHeight,
      maxScrollExtent: maxScrollExtent,
    );

    if (!typewriterAnimate) {
      return;
    }

    final clampedTarget = targetScroll.clamp(0.0, maxScrollExtent);
    final distance = (controller.offset - clampedTarget).abs();
    if (distance <= 1) {
      return;
    }

    final durationMs = math.max(90, math.min(180, (distance * 0.25).round()));
    unawaited(
      controller.animateTo(
        clampedTarget,
        duration: Duration(milliseconds: durationMs),
        curve: Curves.easeOutCubic,
      ),
    );
    return;
  }

  _jumpToKeepCursorInScrollMargin(
    controller: controller,
    cursorTop: cursorGlobalY,
    cursorHeight: cursor.height,
    viewportHeight: viewportHeight,
    maxScrollExtent: controller.position.maxScrollExtent,
  );
}

void _jumpToKeepCursorInScrollMargin({
  required ScrollController controller,
  required double cursorTop,
  required double cursorHeight,
  required double viewportHeight,
  required double maxScrollExtent,
}) {
  final scrollOffset = controller.offset;
  final cursorBottom = cursorTop + cursorHeight;

  if (cursorBottom > scrollOffset + viewportHeight - _scrollMargin) {
    controller.jumpTo((cursorBottom - viewportHeight + _scrollMargin).clamp(0.0, maxScrollExtent));
  } else if (cursorTop < scrollOffset + _scrollMargin) {
    controller.jumpTo((cursorTop - _scrollMargin).clamp(0.0, maxScrollExtent));
  }
}

void _scrollHorizontal({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
}) {
  if (!controller.hasSingleClient || controller.position.maxScrollExtent <= 0) {
    return;
  }

  const scrollMargin = _scrollMargin;
  final cursorX = cursor.x + geometry.horizontalPadding;
  final scrollOffset = controller.offset;
  final viewportWidth = controller.position.viewportDimension;
  final cursorRight = cursorX + 2;

  if (cursorRight > scrollOffset + viewportWidth - scrollMargin) {
    unawaited(
      controller.animateTo(
        cursorRight - viewportWidth + scrollMargin,
        duration: const Duration(milliseconds: 100),
        curve: Curves.easeOut,
      ),
    );
  } else if (cursorX < scrollOffset + scrollMargin) {
    unawaited(
      controller.animateTo(
        (cursorX - scrollMargin).clamp(0, controller.position.maxScrollExtent),
        duration: const Duration(milliseconds: 100),
        curve: Curves.easeOut,
      ),
    );
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
  final absoluteY = geometry.titleAreaHeight + offsets[pageIdx] + targetY;

  if (verticalScrollController.hasSingleClient) {
    final viewportHeight = verticalScrollController.position.viewportDimension;
    final targetOffset = (absoluteY - viewportHeight / 3).clamp(0.0, verticalScrollController.position.maxScrollExtent);
    unawaited(
      verticalScrollController.animateTo(
        targetOffset,
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      ),
    );
  }

  if (horizontalScrollController.hasSingleClient && horizontalScrollController.position.maxScrollExtent > 0) {
    const scrollMargin = 60.0;
    final matchX = targetX + geometry.horizontalPadding;
    final matchRight = matchX + targetWidth;
    final scrollOffset = horizontalScrollController.offset;
    final viewportWidth = horizontalScrollController.position.viewportDimension;

    if (matchRight > scrollOffset + viewportWidth - scrollMargin) {
      unawaited(
        horizontalScrollController.animateTo(
          (matchRight - viewportWidth + scrollMargin).clamp(0.0, horizontalScrollController.position.maxScrollExtent),
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        ),
      );
    } else if (matchX < scrollOffset + scrollMargin) {
      unawaited(
        horizontalScrollController.animateTo(
          (matchX - scrollMargin).clamp(0.0, horizontalScrollController.position.maxScrollExtent),
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        ),
      );
    }
  }
}
