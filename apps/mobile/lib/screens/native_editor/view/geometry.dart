import 'dart:math' as math;

import 'package:typie/screens/native_editor/state/state.dart';

class ContentGeometry {
  const ContentGeometry({
    required this.layout,
    required this.pages,
    required this.titleAreaHeight,
    this.selection,
    this.zoom = 1.0,
  });

  final Layout layout;
  final List<PageSize> pages;
  final double titleAreaHeight;
  final EditorSelection? selection;
  final double zoom;

  static const pageGap = 24.0;
  static const pagePadding = 40.0;
  static const typewriterMinBottomPadding = 48.0;
  static const typewriterPaddingSafety = 1.0;

  double get effectiveZoom => isPaginated ? zoom : 1.0;

  double toDisplayX(double logical) => logical * effectiveZoom;
  double toDisplayY(double logical) => logical * effectiveZoom;
  double toLogicalX(double display) => effectiveZoom <= 0 ? display : display / effectiveZoom;
  double toLogicalY(double display) => effectiveZoom <= 0 ? display : display / effectiveZoom;

  bool get isPaginated => layout is PaginatedLayout;
  double get horizontalPadding => 0;
  double get contentWidth => pageWidthAt(0) + horizontalPadding * 2;
  double get defaultBottomPadding => isPaginated ? toDisplayY(pagePadding) : 200.0;

  double _logicalPageWidthAt(int index) {
    if (layout case PaginatedLayout(:final pageWidth)) {
      return pageWidth;
    }
    return pages.elementAtOrNull(index)?.width ?? 0.0;
  }

  double _logicalPageHeightAt(int index) {
    if (layout case PaginatedLayout(:final pageHeight)) {
      return pageHeight;
    }
    return pages.elementAtOrNull(index)?.height ?? 0.0;
  }

  double pageWidthAt(int index) => toDisplayX(_logicalPageWidthAt(index));
  double pageHeightAt(int index) => toDisplayY(_logicalPageHeightAt(index));

  double contentLeftInset(double viewportWidth) {
    if (viewportWidth <= 0) {
      return 0;
    }
    return ((viewportWidth - contentWidth) / 2).clamp(0.0, double.infinity);
  }

  double contentStartX({required double viewportWidth, required double horizontalScrollOffset}) {
    return contentLeftInset(viewportWidth) + horizontalPadding - horizontalScrollOffset;
  }

  double gapAfterPage(int index) => isPaginated && index < pages.length - 1 ? toDisplayY(pageGap) : 0.0;

  double get pagesContentHeight {
    var total = 0.0;
    for (var i = 0; i < pages.length; i++) {
      total += pageHeightAt(i);
      total += gapAfterPage(i);
    }
    return total;
  }

  double cursorTopInPages(CursorInfo cursor) {
    var top = 0.0;
    for (var i = 0; i < cursor.pageIdx; i++) {
      top += pageHeightAt(i);
      top += gapAfterPage(i);
    }
    return top + toDisplayY(cursor.y);
  }

  double cursorTopInContent(CursorInfo cursor) => cursorTopInPages(cursor) + titleAreaHeight;

  List<double> computeCumulativePageOffsets() {
    final offsets = <double>[0];
    for (var i = 0; i < pages.length; i++) {
      final h = pageHeightAt(i);
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
    final minimumBottomPadding = typewriterEnabled ? typewriterMinBottomPadding : defaultBottomPadding;

    if (!typewriterEnabled || cursor == null) {
      return minimumBottomPadding;
    }

    final cursorHeight = toDisplayY(cursor.height);
    final spaceNeededBelowCursorTop = (1 - typewriterPosition) * (viewportHeight - cursorHeight) + cursorHeight;
    final spaceAvailableBelowCursorTop = (titleAreaHeight + pagesContentHeight - cursorTopInContent(cursor)).clamp(
      0.0,
      double.infinity,
    );

    final requiredPadding = spaceNeededBelowCursorTop - spaceAvailableBelowCursorTop + typewriterPaddingSafety;

    return math.max(minimumBottomPadding, requiredPadding);
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
