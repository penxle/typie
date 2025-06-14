import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/services/keyboard.dart';

class BlockquoteFloatingToolbar extends HookWidget {
  const BlockquoteFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final keyboardType = useValueListenable(scope.keyboardType);

    return FloatingToolbarButton(
      icon: LucideLightIcons.quote,
      onTap: () async {
        scope.bottomToolbarMode.value = BottomToolbarMode.blockquote;
        if (keyboardType == KeyboardType.software) {
          await webViewController?.clearFocus();
        }
      },
    );
  }
}
