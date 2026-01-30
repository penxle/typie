import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/external/models.dart';
import 'package:typie/screens/native_editor/toolbar/floating/file.dart';
import 'package:typie/screens/native_editor/toolbar/floating/image.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorFloatingToolbar extends HookWidget {
  const NativeEditorFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final elements = useValueListenable(scope.externalElements);

    final selectedElement = elements.where((e) => e.isSelected).firstOrNull;

    if (selectedElement == null) {
      return const SizedBox.shrink();
    }

    return switch (selectedElement.data) {
      ImageElementData() => NativeEditorImageFloatingToolbar(element: selectedElement),
      FileElementData() => NativeEditorFileFloatingToolbar(element: selectedElement),
    };
  }
}
