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
    final proseMirrorState = useValueListenable(scope.localProseMirrorState);
    final selected = proseMirrorState?.currentNode?.type == 'blockquote';

    return Row(
      spacing: 8,
      children: selected
          ? [
              FloatingToolbarButton(
                icon: LucideLightIcons.trash_2,
                onTap: () async {
                  await scope.command('delete');
                },
              ),
            ]
          : [
              FloatingToolbarButton(
                icon: LucideLightIcons.quote,
                onTap: () async {
                  scope.bottomToolbarMode.value = BottomToolbarMode.blockquote;
                  if (keyboardType == KeyboardType.software) {
                    await webViewController?.clearFocus();
                  }
                },
              ),
              FloatingToolbarButton(
                icon: LucideLightIcons.text_select,
                onTap: () async {
                  await scope.command('unwrap_node', attrs: {'nodeType': 'blockquote'});
                },
              ),
              FloatingToolbarButton(
                icon: LucideLightIcons.grip_vertical,
                onTap: () async {
                  await scope.command('select_upward_node', attrs: {'nodeType': 'blockquote'});
                },
              ),
            ],
    );
  }
}
