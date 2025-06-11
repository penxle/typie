import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';

class HorizontalRuleFloatingToolbar extends HookWidget {
  const HorizontalRuleFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    return Row(
      spacing: 8,
      children: [
        FloatingToolbarButton(
          icon: TypieIcons.horizontal_rule,
          onTap: () async {
            scope.bottomToolbarMode.value = BottomToolbarMode.horizontalRule;
            await webViewController?.clearFocus();
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () async {
            await scope.command('delete');
          },
        ),
      ],
    );
  }
}
