import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/state/editor_state.dart';

const pageGap = 24.0;

class EditorScrollBehavior {
  const EditorScrollBehavior({
    required this.scrollController,
    required this.viewportHeight,
    required this.viewKeyboardHeight,
  });

  final ScrollController scrollController;
  final double viewportHeight;
  final double viewKeyboardHeight;

  void scrollToCursor(CursorInfo cursor, LayoutInfo layout) {
    if (!cursor.show || viewKeyboardHeight <= 0) {
      return;
    }

    var cursorGlobalY = cursor.y;
    for (var i = 0; i < cursor.pageIdx; i++) {
      final pageHeight = layout.pageHeights.elementAtOrNull(i) ?? 0;
      cursorGlobalY += pageHeight + (layout.isPaginated ? pageGap : 0);
    }

    final effectiveViewportHeight = viewportHeight - viewKeyboardHeight;
    final scrollOffset = scrollController.offset;
    final cursorBottom = cursorGlobalY + cursor.height;

    if (cursorBottom > scrollOffset + effectiveViewportHeight) {
      unawaited(
        scrollController.animateTo(
          cursorBottom - effectiveViewportHeight + 16,
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOut,
        ),
      );
    } else if (cursorGlobalY < scrollOffset) {
      unawaited(
        scrollController.animateTo(
          cursorGlobalY - 16,
          duration: const Duration(milliseconds: 100),
          curve: Curves.easeOut,
        ),
      );
    }
  }
}
