import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/bottom/insert.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/animated_indexed_switcher.dart';

class BottomToolbar extends HookWidget {
  const BottomToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);

    return Container(
      height: keyboardHeight,
      decoration: const BoxDecoration(
        color: AppColors.white,
        border: Border(top: BorderSide(color: AppColors.gray_100)),
      ),
      child: AnimatedIndexedSwitcher(
        index: switch (bottomToolbarMode) {
          BottomToolbarMode.hidden => 0,
          BottomToolbarMode.insert => 1,
        },
        children: const [SizedBox.expand(), InsertBottomToolbar()],
      ),
    );
  }
}
