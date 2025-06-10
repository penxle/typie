import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';

class ListFloatingToolbar extends HookWidget {
  const ListFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return Row(
      spacing: 8,
      children: [
        FloatingToolbarButton(
          icon: LucideLightIcons.dot,
          isActive: proseMirrorState?.isNodeActive('bullet_list') ?? false,
          onTap: () async {
            await scope.command('bullet_list');
          },
        ),
        FloatingToolbarButton(
          icon: LucideLightIcons.hash,
          isActive: proseMirrorState?.isNodeActive('ordered_list') ?? false,
          onTap: () async {
            await scope.command('ordered_list');
          },
        ),
      ],
    );
  }
}
