import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';

class NativeEditorBlockquoteFloatingToolbar extends HookWidget {
  const NativeEditorBlockquoteFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardType = useValueListenable(scope.keyboardType);

    return Row(
      spacing: 8,
      children: [
        FloatingToolbarButton(
          icon: LucideLightIcons.quote,
          onTap: () {
            scope.bottomToolbarMode.value = BottomToolbarMode.blockquote;
            if (keyboardType == KeyboardType.software) {
              scope.dismissKeyboard();
            }
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.text_select,
          onTap: () {
            scope.dispatch({'type': 'toggleBlockquote', 'variant': 'left_line'});
            scope.controller.scrollIntoView();
          },
        ),
      ],
    );
  }
}
