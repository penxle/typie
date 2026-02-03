import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

const pageGap = 24.0;

class EditorScrollBehavior {
  const EditorScrollBehavior({
    required this.scrollController,
    required this.horizontalScrollController,
    required this.horizontalPadding,
    required this.titleHeaderHeight,
    this.typewriterEnabled = false,
    this.typewriterPosition = 0.5,
  });

  final ScrollController scrollController;
  final ScrollController horizontalScrollController;
  final double horizontalPadding;
  final double titleHeaderHeight;
  final bool typewriterEnabled;
  final double typewriterPosition;

  void scrollToCursor(CursorInfo cursor, LayoutInfo layout) {
    if (!cursor.show) {
      return;
    }

    _scrollVertical(cursor, layout);
    _scrollHorizontal(cursor);
  }

  void _scrollVertical(CursorInfo cursor, LayoutInfo layout) {
    if (!scrollController.hasClients) {
      return;
    }

    var cursorGlobalY = cursor.y + titleHeaderHeight;
    for (var i = 0; i < cursor.pageIdx; i++) {
      final pageHeight = layout.pageHeights.elementAtOrNull(i) ?? 0;
      cursorGlobalY += pageHeight + (layout.isPaginated ? pageGap : 0);
    }

    final viewportHeight = scrollController.position.viewportDimension;

    if (typewriterEnabled) {
      final availableRange = viewportHeight - cursor.height;
      final targetScroll = cursorGlobalY - availableRange * typewriterPosition;

      final defaultBottomPadding = layout.isPaginated ? 40.0 : 200.0;
      final bottomPadding = calculateTypewriterBottomPadding(
        defaultPadding: defaultBottomPadding,
        typewriterEnabled: true,
        typewriterPosition: typewriterPosition,
        viewportHeight: viewportHeight,
        layout: layout,
        cursor: cursor,
      );
      var totalContentHeight = titleHeaderHeight;
      for (var i = 0; i < layout.pageCount; i++) {
        totalContentHeight += layout.pageHeights.elementAtOrNull(i) ?? 0;
        if (layout.isPaginated && i < layout.pageCount - 1) {
          totalContentHeight += pageGap;
        }
      }
      totalContentHeight += bottomPadding;
      final maxScrollExtent = math.max<double>(0, totalContentHeight - viewportHeight);

      final clampedTarget = targetScroll.clamp(0.0, maxScrollExtent);
      if ((scrollController.offset - clampedTarget).abs() > 0.5) {
        scrollController.jumpTo(clampedTarget);
      }
      return;
    }

    const scrollMargin = 60.0;
    final scrollOffset = scrollController.offset;
    final cursorBottom = cursorGlobalY + cursor.height;

    if (cursorBottom > scrollOffset + viewportHeight - scrollMargin) {
      scrollController.jumpTo(cursorBottom - viewportHeight + scrollMargin);
    } else if (cursorGlobalY < scrollOffset + scrollMargin) {
      scrollController.jumpTo((cursorGlobalY - scrollMargin).clamp(0, scrollController.position.maxScrollExtent));
    }
  }

  static double calculateTypewriterBottomPadding({
    required double defaultPadding,
    required bool typewriterEnabled,
    required double typewriterPosition,
    required double viewportHeight,
    required LayoutInfo layout,
    required CursorInfo? cursor,
  }) {
    if (!typewriterEnabled || cursor == null) {
      return defaultPadding;
    }

    var totalContentHeight = 0.0;
    for (var i = 0; i < layout.pageCount; i++) {
      totalContentHeight += layout.pageHeights.elementAtOrNull(i) ?? 0.0;
      if (layout.isPaginated && i < layout.pageCount - 1) {
        totalContentHeight += pageGap;
      }
    }

    var cursorTopInDoc = 0.0;
    for (var i = 0; i < cursor.pageIdx; i++) {
      cursorTopInDoc += layout.pageHeights.elementAtOrNull(i) ?? 0.0;
      if (layout.isPaginated) {
        cursorTopInDoc += pageGap;
      }
    }
    cursorTopInDoc += cursor.y;

    final spaceNeededBelow = (1 - typewriterPosition) * (viewportHeight - cursor.height) + 2 * cursor.height;
    final contentBelow = totalContentHeight - cursorTopInDoc;
    final extraPadding = spaceNeededBelow - contentBelow;

    return math.max(defaultPadding, extraPadding);
  }

  void _scrollHorizontal(CursorInfo cursor) {
    if (!horizontalScrollController.hasClients || horizontalScrollController.position.maxScrollExtent <= 0) {
      return;
    }

    const scrollMargin = 60.0;
    final cursorX = cursor.x + horizontalPadding;
    final scrollOffset = horizontalScrollController.offset;
    final viewportWidth = horizontalScrollController.position.viewportDimension;
    final cursorRight = cursorX + 2;

    if (cursorRight > scrollOffset + viewportWidth - scrollMargin) {
      unawaited(
        horizontalScrollController.animateTo(
          cursorRight - viewportWidth + scrollMargin,
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOut,
        ),
      );
    } else if (cursorX < scrollOffset + scrollMargin) {
      unawaited(
        horizontalScrollController.animateTo(
          (cursorX - scrollMargin).clamp(0, horizontalScrollController.position.maxScrollExtent),
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOut,
        ),
      );
    }
  }
}
