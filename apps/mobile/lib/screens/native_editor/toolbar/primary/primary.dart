import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/icon.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';

class NativeEditorPrimaryToolbar extends HookWidget {
  const NativeEditorPrimaryToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardType = useValueListenable(scope.keyboardType);
    final secondaryToolbarMode = useValueListenable(scope.secondaryToolbarMode);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);

    return Container(
      height: 48,
      decoration: BoxDecoration(
        color: context.colors.surfaceDefault,
        border: Border(top: BorderSide(color: context.colors.borderSubtle)),
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
                    onTap: () {
                      if (bottomToolbarMode == BottomToolbarMode.insert) {
                        switch (keyboardType) {
                          case KeyboardType.software:
                            scope.requestFocus();
                          case KeyboardType.hardware:
                            scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
                        }
                      } else {
                        scope.bottomToolbarMode.value = BottomToolbarMode.insert;
                        if (keyboardType == KeyboardType.software) {
                          scope.dismissKeyboard();
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
                    onTap: () {
                      scope.dispatch({'type': 'insertImage'});
                      scope.controller.scrollIntoView();
                      scope.requestFocus();
                    },
                  ),
                  IconToolbarButton(
                    icon: LucideLightIcons.undo,
                    onTap: () {
                      scope.dispatch({'type': 'undo'});
                      scope.controller.scrollIntoView();
                    },
                  ),
                  IconToolbarButton(
                    icon: LucideLightIcons.redo,
                    onTap: () {
                      scope.dispatch({'type': 'redo'});
                      scope.controller.scrollIntoView();
                    },
                  ),
                ],
              ),
            ),
          ),
          IconToolbarButton(
            icon: LucideLightIcons.chevron_left,
            isRepeatable: true,
            onTap: () {
              scope.commitComposing();
              scope.requestFocus();
              scope.dispatch({'type': 'navigate', 'direction': 'left', 'extend': false});
              scope.controller.scrollIntoView();
            },
          ),
          IconToolbarButton(
            icon: LucideLightIcons.chevron_right,
            isRepeatable: true,
            onTap: () {
              scope.commitComposing();
              scope.requestFocus();
              scope.dispatch({'type': 'navigate', 'direction': 'right', 'extend': false});
              scope.controller.scrollIntoView();
            },
          ),
          AnimatedIndexedSwitcher(
            index: bottomToolbarMode == BottomToolbarMode.hidden && secondaryToolbarMode == SecondaryToolbarMode.hidden
                ? 0
                : 1,
            children: [
              IconToolbarButton(
                icon: LucideLightIcons.keyboard_off,
                onTap: () {
                  if (keyboardType == KeyboardType.software) {
                    scope.clearFocus();
                  }
                },
              ),
              IconToolbarButton(
                icon: LucideLightIcons.circle_x,
                onTap: () {
                  switch (keyboardType) {
                    case KeyboardType.software:
                      scope.requestFocus();
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
