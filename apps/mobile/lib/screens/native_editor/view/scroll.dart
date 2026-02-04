import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';

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
  _scrollHorizontal(controller: horizontalController, geometry: geometry, cursor: cursor);
}

void _scrollVertical({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
  required bool typewriterEnabled,
  required double typewriterPosition,
}) {
  if (!controller.hasClients) {
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

    final clampedTarget = targetScroll.clamp(0.0, maxScrollExtent);
    if ((controller.offset - clampedTarget).abs() > 0.5) {
      controller.jumpTo(clampedTarget);
    }
    return;
  }

  const scrollMargin = 60.0;
  final scrollOffset = controller.offset;
  final cursorBottom = cursorGlobalY + cursor.height;

  if (cursorBottom > scrollOffset + viewportHeight - scrollMargin) {
    controller.jumpTo(cursorBottom - viewportHeight + scrollMargin);
  } else if (cursorGlobalY < scrollOffset + scrollMargin) {
    controller.jumpTo((cursorGlobalY - scrollMargin).clamp(0, controller.position.maxScrollExtent));
  }
}

void _scrollHorizontal({
  required ScrollController controller,
  required ContentGeometry geometry,
  required CursorInfo cursor,
}) {
  if (!controller.hasClients || controller.position.maxScrollExtent <= 0) {
    return;
  }

  const scrollMargin = 60.0;
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
