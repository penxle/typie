import 'dart:math' as math;

import 'package:typie/screens/native_editor/state/state.dart';

class ContentGeometry {
  const ContentGeometry({required this.layout, required this.pages, required this.titleAreaHeight, this.selection});

  final Layout layout;
  final List<PageSize> pages;
  final double titleAreaHeight;
  final EditorSelection? selection;

  static const pageGap = 24.0;
  static const pagePadding = 40.0;
  static const continuousPageMargin = 20.0;

  bool get isPaginated => layout is PaginatedLayout;
  double get horizontalPadding => isPaginated ? pagePadding : 0.0;
  double get contentWidth => (pages.firstOrNull?.width ?? 0) + horizontalPadding * 2;
  double get defaultBottomPadding => isPaginated ? pagePadding : 200.0;
  double get trailingBottomMargin =>
      layout is PaginatedLayout ? (layout as PaginatedLayout).pageMarginBottom : continuousPageMargin;

  double contentLeftInset(double viewportWidth) {
    if (viewportWidth <= 0) {
      return 0;
    }
    return ((viewportWidth - contentWidth) / 2).clamp(0.0, double.infinity);
  }

  double contentStartX({required double viewportWidth, required double horizontalScrollOffset}) {
    return contentLeftInset(viewportWidth) + horizontalPadding - horizontalScrollOffset;
  }

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

  double? get collapsedSelectionHandleHeight {
    if (!(selection?.collapsed ?? false)) {
      return null;
    }
    return selection?.headBounds?.height ?? selection?.anchorBounds?.height;
  }

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

    final cursorHeight = cursor.height;
    final handleHeight = collapsedSelectionHandleHeight ?? cursorHeight;
    final cursorLeading = math.max(0, handleHeight - cursorHeight);
    final spaceNeededBelowCursorTop = (1 - typewriterPosition) * (viewportHeight - cursorHeight) + cursorHeight;
    final intrinsicSpaceBelowLastLine = trailingBottomMargin + cursorHeight + cursorLeading;
    final requiredPadding = spaceNeededBelowCursorTop - intrinsicSpaceBelowLastLine;

    return math.max(defaultBottomPadding, requiredPadding);
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
