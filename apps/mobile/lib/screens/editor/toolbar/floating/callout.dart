import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';

class CalloutFloatingToolbar extends HookWidget {
  const CalloutFloatingToolbar({super.key});

  static const calloutTypes = ['info', 'success', 'warning', 'danger'];
  static const calloutIcons = {
    'info': LucideLightIcons.info,
    'success': LucideLightIcons.circle_check,
    'warning': LucideLightIcons.circle_alert,
    'danger': LucideLightIcons.triangle_alert,
  };

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);
    final selected = proseMirrorState?.currentNode?.type == 'callout';

    final currentType = proseMirrorState?.getNodeAttributes('callout')?['type'] as String? ?? 'info';
    final currentIcon = calloutIcons[currentType] ?? LucideLightIcons.info;

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
                icon: currentIcon,
                onTap: () async {
                  final currentIndex = calloutTypes.indexOf(currentType);
                  final nextType = calloutTypes[(currentIndex + 1) % calloutTypes.length];

                  await scope.command('update_node_attribute', attrs: {'nodeType': 'callout', 'type': nextType});
                },
              ),
              FloatingToolbarButton(
                icon: LucideLightIcons.text_select,
                onTap: () async {
                  await scope.command('unwrap_node', attrs: {'nodeType': 'callout'});
                },
              ),
              FloatingToolbarButton(
                icon: LucideLightIcons.grip_vertical,
                onTap: () async {
                  await scope.command('select_upward_node', attrs: {'nodeType': 'callout'});
                },
              ),
            ],
    );
  }
}
