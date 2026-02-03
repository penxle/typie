import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/toolbar/bottom/bottom.dart';
import 'package:typie/screens/native_editor/toolbar/primary/primary.dart';
import 'package:typie/screens/native_editor/toolbar/scope.dart';
import 'package:typie/screens/native_editor/toolbar/secondary/secondary.dart';
import 'package:typie/services/keyboard.dart';

class NativeEditorToolbar extends HookWidget {
  const NativeEditorToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = NativeEditorToolbarScope.of(context);
    final isKeyboardVisible = useValueListenable(scope.isKeyboardVisible);
    final keyboardHeight = useValueListenable(scope.keyboardHeight);
    final keyboardType = useValueListenable(scope.keyboardType);
    final isEditorFocused = useValueListenable(scope.isEditorFocused);
    final bottomToolbarMode = useValueListenable(scope.bottomToolbarMode);

    if (!isEditorFocused) {
      if (isKeyboardVisible) {
        return SizedBox(height: keyboardHeight);
      }
      return const SizedBox.shrink();
    }

    if (keyboardType == KeyboardType.software && !isKeyboardVisible && bottomToolbarMode == BottomToolbarMode.hidden) {
      return const SizedBox.shrink();
    }

    return const Column(
      children: [NativeEditorSecondaryToolbar(), NativeEditorPrimaryToolbar(), NativeEditorBottomToolbar()],
    );
  }
}
