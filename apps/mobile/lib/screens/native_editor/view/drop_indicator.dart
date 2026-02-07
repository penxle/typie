import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/view/scope.dart';

class DropIndicator extends HookWidget {
  const DropIndicator({super.key, required this.pageIdx});

  final int pageIdx;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final indicator = useListenableSelector(scope.controller, () {
      final info = scope.controller.state.dropIndicator;
      return info?.pageIdx == pageIdx ? info : null;
    });

    if (indicator == null) {
      return const SizedBox.shrink();
    }

    return Positioned(
      left: indicator.x,
      top: indicator.y,
      width: indicator.width,
      height: indicator.height,
      child: Container(color: Theme.of(context).primaryColor.withValues(alpha: 0.5)),
    );
  }
}
