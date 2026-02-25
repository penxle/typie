import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

typedef DocumentOverlayBuilder = Widget Function(BuildContext context, DocumentOverlayViewport viewport);

class DocumentOverlayViewport {
  const DocumentOverlayViewport({
    required this.geometry,
    required this.pageOffsets,
    required this.verticalScrollOffset,
    required this.horizontalScrollOffset,
    required this.viewportWidth,
  });

  final ContentGeometry geometry;
  final List<double> pageOffsets;
  final double verticalScrollOffset;
  final double horizontalScrollOffset;
  final double viewportWidth;

  bool hasPage(int pageIdx) {
    return pageIdx >= 0 && pageIdx < geometry.pages.length;
  }

  Rect pageRect(int pageIdx) {
    final clamped = pageIdx.clamp(0, geometry.pages.length - 1);
    final left = geometry.contentStartX(viewportWidth: viewportWidth, horizontalScrollOffset: horizontalScrollOffset);
    final top = geometry.titleAreaHeight + pageOffsets[clamped] - verticalScrollOffset;
    return Rect.fromLTWH(left, top, geometry.pageWidthAt(clamped), geometry.pageHeightAt(clamped));
  }
}

class DocumentOverlayLayer extends HookWidget {
  const DocumentOverlayLayer({required this.builder, super.key});

  final DocumentOverlayBuilder builder;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final state = useListenable(scope.controller);
    useListenable(scope.verticalScrollController);
    useListenable(scope.horizontalScrollController);
    final titleAreaHeight = useValueListenable(scope.titleAreaHeight);
    useValueListenable(scope.displayZoom);
    final layout = state.state.layout;
    final pages = state.state.pages;
    if (layout == null || pages.isEmpty) {
      return const SizedBox.shrink();
    }

    final geometry = ContentGeometry(
      layout: layout,
      pages: pages,
      titleAreaHeight: titleAreaHeight,
      zoom: scope.displayZoom.value,
    );
    final pageOffsets = geometry.computeCumulativePageOffsets();
    final verticalScrollOffset = resolveScrollOffset(scope.verticalScrollController);
    final horizontalMetrics = resolveHorizontalScrollMetrics(
      controller: scope.horizontalScrollController,
      contentWidth: geometry.contentWidth,
      fallbackViewportDimension: MediaQuery.sizeOf(context).width,
    );
    final viewportWidth = horizontalMetrics.viewportDimension;
    final horizontalScrollOffset = horizontalMetrics.scrollOffset;

    final viewport = DocumentOverlayViewport(
      geometry: geometry,
      pageOffsets: pageOffsets,
      verticalScrollOffset: verticalScrollOffset,
      horizontalScrollOffset: horizontalScrollOffset,
      viewportWidth: viewportWidth,
    );

    return Positioned.fill(
      child: Stack(clipBehavior: Clip.none, children: [builder(context, viewport)]),
    );
  }
}
