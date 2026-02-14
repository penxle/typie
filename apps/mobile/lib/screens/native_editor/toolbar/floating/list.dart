import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';

class NativeEditorListFloatingToolbar extends HookWidget {
  const NativeEditorListFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final floatingContext = useValueListenable(scope.floatingContext);

    final isBullet = floatingContext == 'in_bullet_list';
    final isOrdered = floatingContext == 'in_ordered_list';

    return Row(
      spacing: 8,
      children: [
        FloatingToolbarButton(
          icon: LucideLightIcons.dot,
          isActive: isBullet,
          onTap: () {
            scope.dispatch({'type': 'toggleBulletList'});
            scope.controller.scrollIntoView();
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.hash,
          isActive: isOrdered,
          onTap: () {
            scope.dispatch({'type': 'toggleOrderedList'});
            scope.controller.scrollIntoView();
          },
        ),
        const SizedBox.shrink(),
        FloatingToolbarButton(
          icon: LucideLightIcons.indent_increase,
          isActive: isBullet || isOrdered,
          onTap: () {
            scope.dispatch({'type': 'indent'});
            scope.controller.scrollIntoView();
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.indent_decrease,
          isActive: isBullet || isOrdered,
          onTap: () {
            scope.dispatch({'type': 'outdent'});
            scope.controller.scrollIntoView();
          },
        ),
      ],
    );
  }
}
