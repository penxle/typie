import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/bottom/blockquote.dart';
import 'package:typie/screens/editor/toolbar/bottom/horizontal_rule.dart';
import 'package:typie/screens/editor/toolbar/bottom/insert.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';

class BottomToolbar extends HookWidget {
  const BottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final keyboardType = useValueListenable(scope.keyboardType);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);

    final mediaQuery = MediaQuery.of(context);

    return AnimatedContainer(
      duration: const Duration(milliseconds: 300),
      curve: Curves.ease,
      height: switch (keyboardType) {
        KeyboardType.software => keyboardHeight,
        KeyboardType.hardware => switch (bottomToolbarMode) {
          BottomToolbarMode.hidden => mediaQuery.viewPadding.bottom,
          _ => mediaQuery.viewPadding.bottom + mediaQuery.size.height * 0.2,
        },
      },
      decoration: BoxDecoration(
        color: context.colors.surfaceDefault,
        border: Border(top: BorderSide(color: context.colors.borderSubtle)),
      ),
      child: AnimatedIndexedSwitcher(
        index: switch (bottomToolbarMode) {
          BottomToolbarMode.hidden => 0,
          BottomToolbarMode.insert => 1,
          BottomToolbarMode.horizontalRule => 2,
          BottomToolbarMode.blockquote => 3,
        },
        children: const [
          SizedBox.expand(),
          InsertBottomToolbar(),
          HorizontalRuleBottomToolbar(),
          BlockquoteBottomToolbar(),
        ],
      ),
    );
  }
}
