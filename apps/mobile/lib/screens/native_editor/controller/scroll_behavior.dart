import 'dart:async';

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
  });

  final ScrollController scrollController;
  final ScrollController horizontalScrollController;
  final double horizontalPadding;
  final double titleHeaderHeight;

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

    const scrollMargin = 60.0;
    final scrollOffset = scrollController.offset;
    final viewportHeight = scrollController.position.viewportDimension;
    final cursorBottom = cursorGlobalY + cursor.height;

    if (cursorBottom > scrollOffset + viewportHeight - scrollMargin) {
      unawaited(
        scrollController.animateTo(
          cursorBottom - viewportHeight + scrollMargin,
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOut,
        ),
      );
    } else if (cursorGlobalY < scrollOffset + scrollMargin) {
      unawaited(
        scrollController.animateTo(
          (cursorGlobalY - scrollMargin).clamp(0, scrollController.position.maxScrollExtent),
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOut,
        ),
      );
    }
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
