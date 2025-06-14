import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/icon.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';

class PrimaryToolbar extends HookWidget {
  const PrimaryToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final keyboardType = useValueListenable(scope.keyboardType);
    final secondaryToolbarMode = useValueListenable(scope.secondaryToolbarMode);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);

    return Container(
      height: 48,
      decoration: const BoxDecoration(
        color: AppColors.white,
        border: Border(top: BorderSide(color: AppColors.gray_100)),
      ),
      padding: const Pad(right: 8),
      child: Row(
        spacing: 8,
        children: [
          Expanded(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              physics: const AlwaysScrollableScrollPhysics(),
              padding: const Pad(left: 8),
              child: Row(
                spacing: 4,
                children: [
                  IconToolbarButton(
                    icon: LucideLightIcons.plus,
                    isActive: bottomToolbarMode == BottomToolbarMode.insert,
                    onTap: () async {
                      if (bottomToolbarMode == BottomToolbarMode.insert) {
                        switch (keyboardType) {
                          case KeyboardType.software:
                            await webViewController?.requestFocus();
                          case KeyboardType.hardware:
                            scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
                        }
                      } else {
                        scope.bottomToolbarMode.value = BottomToolbarMode.insert;
                        if (keyboardType == KeyboardType.software) {
                          await webViewController?.clearFocus();
                        }
                      }
                    },
                  ),
                  IconToolbarButton(
                    icon: LucideLightIcons.type_,
                    isActive: secondaryToolbarMode != SecondaryToolbarMode.hidden,
                    onTap: () {
                      scope.secondaryToolbarMode.value = secondaryToolbarMode == SecondaryToolbarMode.hidden
                          ? SecondaryToolbarMode.text
                          : SecondaryToolbarMode.hidden;
                    },
                  ),
                  IconToolbarButton(
                    icon: LucideLightIcons.image,
                    onTap: () async {
                      await scope.command('image');
                      await webViewController?.requestFocus();
                    },
                  ),
                  IconToolbarButton(
                    icon: LucideLightIcons.undo,
                    onTap: () async {
                      await scope.command('undo');
                    },
                  ),
                  IconToolbarButton(
                    icon: LucideLightIcons.redo,
                    onTap: () async {
                      await scope.command('redo');
                    },
                  ),
                ],
              ),
            ),
          ),
          IconToolbarButton(
            icon: LucideLightIcons.chevron_left,
            isRepeatable: true,
            onTap: () async {
              await webViewController?.requestFocus();
              await scope.webViewController.value?.emitEvent('caret', {'direction': -1});
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.chevron_right,
            isRepeatable: true,
            onTap: () async {
              await webViewController?.requestFocus();
              await scope.webViewController.value?.emitEvent('caret', {'direction': 1});
            },
          ),
          AnimatedIndexedSwitcher(
            index: bottomToolbarMode == BottomToolbarMode.hidden && secondaryToolbarMode == SecondaryToolbarMode.hidden
                ? 0
                : 1,
            children: [
              IconToolbarButton(
                icon: LucideLightIcons.keyboard_off,
                onTap: () async {
                  if (keyboardType == KeyboardType.software) {
                    await webViewController?.clearFocus();
                  }
                },
              ),
              IconToolbarButton(
                icon: LucideLightIcons.circle_x,
                onTap: () async {
                  switch (keyboardType) {
                    case KeyboardType.software:
                      await webViewController?.requestFocus();
                    case KeyboardType.hardware:
                      scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
                  }

                  scope.secondaryToolbarMode.value = SecondaryToolbarMode.hidden;
                },
              ),
            ],
          ),
        ],
      ),
    );
  }
}
