import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/toolbar/buttons/icon.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';
import 'package:typie/widgets/responsive_container.dart';

class NativeEditorPrimaryToolbar extends HookWidget {
  const NativeEditorPrimaryToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final keyboardType = useValueListenable(scope.keyboardType);
    final secondaryToolbarMode = useValueListenable(scope.secondaryToolbarMode);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);
    const trailingActionSize = 36.0;
    final isToolbarCollapsed =
        bottomToolbarMode == BottomToolbarMode.hidden && secondaryToolbarMode == SecondaryToolbarMode.hidden;
    final shouldShowTrailingAction = keyboardType != KeyboardType.hardware || !isToolbarCollapsed || isKeyboardVisible;

    return Container(
      height: 48,
      decoration: BoxDecoration(
        color: context.colors.surfaceDefault,
        border: Border(top: BorderSide(color: context.colors.borderSubtle)),
      ),
      child: ResponsiveContainer(
        alignment: Alignment.center,
        child: Padding(
          padding: const Pad(right: 8),
          child: Row(
            spacing: 8,
            children: [
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  physics: const AlwaysScrollableScrollPhysics(),
                  clipBehavior: Clip.none,
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
                            if (isKeyboardVisible) {
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
              ClipRect(
                child: AnimatedAlign(
                  duration: const Duration(milliseconds: 150),
                  curve: Curves.easeOutCubic,
                  alignment: Alignment.centerRight,
                  widthFactor: shouldShowTrailingAction ? 1 : 0,
                  child: SizedBox(
                    width: trailingActionSize,
                    child: IgnorePointer(
                      ignoring: !shouldShowTrailingAction,
                      child: AnimatedIndexedSwitcher(
                        index: isToolbarCollapsed ? 0 : 1,
                        children: [
                          IconToolbarButton(
                            icon: LucideLightIcons.keyboard_off,
                            onTap: () {
                              switch (keyboardType) {
                                case KeyboardType.software:
                                case KeyboardType.hardware:
                                  scope.dismissKeyboard();
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
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
