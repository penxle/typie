import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/native_editor/toolbar/bottom/blockquote.dart';
import 'package:typie/screens/native_editor/toolbar/bottom/horizontal_rule.dart';
import 'package:typie/screens/native_editor/toolbar/bottom/insert.dart';
import 'package:typie/screens/native_editor/toolbar/bottom/table.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';
import 'package:typie/widgets/responsive_container.dart';

class NativeEditorBottomToolbar extends HookWidget {
  const NativeEditorBottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final keyboardType = useValueListenable(scope.keyboardType);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);
    final lastSoftwareKeyboardHeight = useRef<double>(0);

    final mediaQuery = MediaQuery.of(context);
    final expandedHeightFallback = mediaQuery.viewPadding.bottom + mediaQuery.size.height * 0.2;
    if (keyboardType == KeyboardType.software && keyboardHeight > lastSoftwareKeyboardHeight.value) {
      lastSoftwareKeyboardHeight.value = keyboardHeight;
    }
    // 소프트 키보드에서만 마지막 높이를 재사용하고, 확장 높이는 fallback 이상으로 보장한다.
    final softwareExpandedHeight =
        keyboardType == KeyboardType.software && lastSoftwareKeyboardHeight.value > expandedHeightFallback
        ? lastSoftwareKeyboardHeight.value
        : expandedHeightFallback;
    final hardwareVisibleHeight = switch (bottomToolbarMode) {
      BottomToolbarMode.hidden => keyboardHeight,
      _ => keyboardHeight > expandedHeightFallback ? keyboardHeight : expandedHeightFallback,
    };

    return AnimatedContainer(
      duration: const Duration(milliseconds: 300),
      curve: Curves.ease,
      height: isKeyboardVisible
          ? switch (keyboardType) {
              KeyboardType.software => switch (bottomToolbarMode) {
                BottomToolbarMode.hidden => keyboardHeight,
                _ => softwareExpandedHeight,
              },
              KeyboardType.hardware => hardwareVisibleHeight,
            }
          : switch (keyboardType) {
              KeyboardType.software => switch (bottomToolbarMode) {
                BottomToolbarMode.hidden => mediaQuery.viewPadding.bottom,
                _ => softwareExpandedHeight,
              },
              KeyboardType.hardware => switch (bottomToolbarMode) {
                BottomToolbarMode.hidden => mediaQuery.viewPadding.bottom,
                _ => mediaQuery.viewPadding.bottom + mediaQuery.size.height * 0.2,
              },
            },
      decoration: BoxDecoration(
        color: context.colors.surfaceDefault,
        border: Border(top: BorderSide(color: context.colors.borderSubtle)),
      ),
      child: ResponsiveContainer(
        child: AnimatedIndexedSwitcher(
          index: switch (bottomToolbarMode) {
            BottomToolbarMode.hidden => 0,
            BottomToolbarMode.insert => 1,
            BottomToolbarMode.horizontalRule => 2,
            BottomToolbarMode.blockquote => 3,
            BottomToolbarMode.tableSize => 4,
          },
          children: const [
            SizedBox.expand(),
            NativeEditorInsertBottomToolbar(),
            NativeEditorHorizontalRuleBottomToolbar(),
            NativeEditorBlockquoteBottomToolbar(),
            NativeEditorTableSizeBottomToolbar(),
          ],
        ),
      ),
    );
  }
}
