import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/external/embed.dart';
import 'package:typie/screens/native_editor/external/file.dart';
import 'package:typie/screens/native_editor/external/image.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class ElementOverlay extends HookWidget {
  const ElementOverlay({required this.pageIndex, super.key});

  final int pageIndex;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final elements = useValueListenable(scope.externalElements);

    final pageElements = elements.where((e) => e.pageIdx == pageIndex).toList();

    if (pageElements.isEmpty) {
      return const SizedBox.shrink();
    }

    return Positioned.fill(
      child: Stack(
        children: pageElements.map((element) {
          return Positioned(
            key: ValueKey(element.nodeId),
            left: element.bounds.x,
            top: element.bounds.y,
            width: element.bounds.width,
            child: _ExternalElementWrapper(element: element),
          );
        }).toList(),
      ),
    );
  }
}

class _ExternalElementWrapper extends HookWidget {
  const _ExternalElementWrapper({required this.element});

  final ExternalElement element;

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final contentKey = useMemoized(GlobalKey.new);
    final reportedHeight = useRef<double?>(null);

    useListenable(scope.uploadManager);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final renderBox = contentKey.currentContext?.findRenderObject() as RenderBox?;
        final height = renderBox?.size.height;
        if (height != null && height > 0 && height != reportedHeight.value) {
          reportedHeight.value = height;
          scope.dispatch({'type': 'setExternalElementHeight', 'nodeId': element.nodeId, 'height': height});
        }
      });
      return null;
    });

    final content = KeyedSubtree(key: contentKey, child: _buildElementWidget());

    return Stack(
      children: [
        content,
        if (element.isSelected)
          Positioned.fill(
            child: IgnorePointer(child: Container(color: const Color.fromRGBO(153, 204, 255, 0.3))),
          ),
      ],
    );
  }

  Widget _buildElementWidget() {
    return switch (element.data) {
      ImageElementData() => ImageWidget(element: element),
      FileElementData() => FileWidget(element: element),
      EmbedElementData() => EmbedWidget(element: element),
    };
  }
}
