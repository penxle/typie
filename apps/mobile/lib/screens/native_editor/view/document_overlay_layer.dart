import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

typedef DocumentOverlayBuilder = Widget Function(BuildContext context, DocumentOverlayViewport viewport);

class DocumentOverlayViewport {
  const DocumentOverlayViewport({
    required this.geometry,
    required this.pageOffsets,
    required this.verticalScrollOffset,
    required this.horizontalScrollOffset,
  });

  final ContentGeometry geometry;
  final List<double> pageOffsets;
  final double verticalScrollOffset;
  final double horizontalScrollOffset;

  bool hasPage(int pageIdx) {
    return pageIdx >= 0 && pageIdx < geometry.layout.pages.length;
  }

  Rect pageRect(int pageIdx) {
    final clamped = pageIdx.clamp(0, geometry.layout.pages.length - 1);
    final page = geometry.layout.pages[clamped];
    final left = geometry.horizontalPadding - horizontalScrollOffset;
    final top = geometry.titleAreaHeight + pageOffsets[clamped] - verticalScrollOffset;
    return Rect.fromLTWH(left, top, page.width, page.height);
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
    final layout = state.state.layout;
    if (layout == null || layout.pages.isEmpty) {
      return const SizedBox.shrink();
    }

    final geometry = ContentGeometry(layout: layout, titleAreaHeight: titleAreaHeight);
    final pageOffsets = geometry.computeCumulativePageOffsets();
    final verticalScrollOffset = scope.verticalScrollController.hasClients
        ? scope.verticalScrollController.offset
        : 0.0;
    final horizontalScrollOffset = scope.horizontalScrollController.hasClients
        ? scope.horizontalScrollController.offset
        : 0.0;

    final viewport = DocumentOverlayViewport(
      geometry: geometry,
      pageOffsets: pageOffsets,
      verticalScrollOffset: verticalScrollOffset,
      horizontalScrollOffset: horizontalScrollOffset,
    );

    return Positioned.fill(
      child: Stack(clipBehavior: Clip.none, children: [builder(context, viewport)]),
    );
  }
}
