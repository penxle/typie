import 'package:flutter/material.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorFoldFloatingToolbar extends StatelessWidget {
  const NativeEditorFoldFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);

    return FloatingToolbarButton(
      icon: LucideLightIcons.text_select,
      onTap: () {
        scope.dispatch({'type': 'unwrapFold'});
        scope.controller.scrollIntoView();
      },
    );
  }
}
