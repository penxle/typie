import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/bottom/bottom.dart';
import 'package:typie/screens/editor/toolbar/primary/primary.dart';
import 'package:typie/screens/editor/toolbar/secondary/secondary.dart';
import 'package:typie/services/keyboard.dart';

class EditorToolbar extends HookWidget {
  const EditorToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final keyboardType = useValueListenable(scope.keyboardType);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);

    if (keyboardType == KeyboardType.software && !isKeyboardVisible && bottomToolbarMode == BottomToolbarMode.hidden) {
      return const SizedBox.shrink();
    }

    return const Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [SecondaryToolbar(), PrimaryToolbar(), BottomToolbar()],
    );
  }
}
