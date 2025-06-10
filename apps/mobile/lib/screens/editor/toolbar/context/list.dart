import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/icon.dart';
import 'package:typie/screens/editor/toolbar/context/node.dart';

class ListToolbar extends HookWidget {
  const ListToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return NodeToolbar(
      withDelete: false,
      children: [
        IconToolbarButton(
          icon: LucideLightIcons.list,
          isActive: proseMirrorState?.isNodeActive('bullet_list') ?? false,
          onTap: () async {
            await scope.command('bullet_list');
          },
        ),
        IconToolbarButton(
          icon: LucideLightIcons.list_ordered,
          isActive: proseMirrorState?.isNodeActive('ordered_list') ?? false,
          onTap: () async {
            await scope.command('ordered_list');
          },
        ),
      ],
    );
  }
}
