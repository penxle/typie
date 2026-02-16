import 'dart:math' as math;

import 'package:typie/screens/native_editor/state/state.dart';

class ContentGeometry {
  const ContentGeometry({required this.layout, required this.pages, required this.titleAreaHeight});

  final Layout layout;
  final List<PageSize> pages;
  final double titleAreaHeight;

  static const pageGap = 24.0;
  static const pagePadding = 40.0;

  bool get isPaginated => layout is PaginatedLayout;
  double get horizontalPadding => isPaginated ? pagePadding : 0.0;
  double get defaultBottomPadding => isPaginated ? pagePadding : 200.0;

  double gapAfterPage(int index) => isPaginated && index < pages.length - 1 ? pageGap : 0.0;

  double get pagesContentHeight {
    var total = 0.0;
    for (var i = 0; i < pages.length; i++) {
      total += pages.elementAtOrNull(i)?.height ?? 0.0;
      total += gapAfterPage(i);
    }
    return total;
  }

  double cursorTopInPages(CursorInfo cursor) {
    var top = 0.0;
    for (var i = 0; i < cursor.pageIdx; i++) {
      top += pages.elementAtOrNull(i)?.height ?? 0.0;
      top += gapAfterPage(i);
    }
    return top + cursor.y;
  }

  double cursorTopInContent(CursorInfo cursor) => cursorTopInPages(cursor) + titleAreaHeight;

  List<double> computeCumulativePageOffsets() {
    final offsets = <double>[0];
    for (var i = 0; i < pages.length; i++) {
      final h = pages.elementAtOrNull(i)?.height ?? 0.0;
      offsets.add(offsets.last + h + gapAfterPage(i));
    }
    return offsets;
  }

  double bottomPadding({
    required double viewportHeight,
    CursorInfo? cursor,
    bool typewriterEnabled = false,
    double typewriterPosition = 0.5,
  }) {
    if (!typewriterEnabled || cursor == null) {
      return defaultBottomPadding;
    }

    final cursorTopInDoc = cursorTopInPages(cursor);
    final spaceNeededBelow = (1 - typewriterPosition) * (viewportHeight - cursor.height) + 2 * cursor.height;
    final contentBelow = pagesContentHeight - cursorTopInDoc;
    final extraPadding = spaceNeededBelow - contentBelow;

    return math.max(defaultBottomPadding, extraPadding);
  }

  double totalContentHeight({
    required double viewportHeight,
    CursorInfo? cursor,
    bool typewriterEnabled = false,
    double typewriterPosition = 0.5,
  }) {
    return titleAreaHeight +
        pagesContentHeight +
        bottomPadding(
          viewportHeight: viewportHeight,
          cursor: cursor,
          typewriterEnabled: typewriterEnabled,
          typewriterPosition: typewriterPosition,
        );
  }
}
