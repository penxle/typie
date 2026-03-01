import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';

class NativeEditorHorizontalRuleFloatingToolbar extends HookWidget {
  const NativeEditorHorizontalRuleFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardType = useValueListenable(scope.keyboardType);

    return Row(
      spacing: 8,
      children: [
        FloatingToolbarButton(
          icon: TypieIcons.horizontal_rule,
          onTap: () {
            scope.bottomToolbarMode.value = BottomToolbarMode.horizontalRule;
            if (keyboardType == KeyboardType.software) {
              scope.dismissKeyboard();
            }
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () {
            scope.dispatch({'type': 'deleteSelection'});
            scope.controller.scrollIntoView();
          },
        ),
      ],
    );
  }
}
